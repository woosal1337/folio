//! Agent eval-fixture format.
//!
//! Each `.json` fixture under `crates/attune-core/evals/` describes
//! one canned scenario the agent suite is expected to handle:
//!
//! ```json
//! {
//!   "id": "summarize-pricing-sync",
//!   "agent_id": "summarize",
//!   "transcript": "Lila: ...\nAlex: ...",
//!   "expectations": {
//!     "must_contain": ["pricing", "Lila"],
//!     "must_not_contain": ["I cannot"],
//!     "min_words": 30,
//!     "max_words": 250,
//!     "must_not_speculate": true
//!   }
//! }
//! ```
//!
//! v2 roadmap finding 036 / GET-88 prerequisite. This PR lays the
//! schema + loader + tiny check harness so the next PR can wire a
//! `cargo test --features=eval-runner` (or `attune-cli eval`) path
//! that actually fires the agents against the fixtures. The bulk of
//! that work — recorded provider tapes, CI gating — stays as a
//! follow-up; we ship the contract here so prompts can ship behind
//! a versioned eval surface starting today.
//!
//! ## Four-quadrant grading rubric (GET-199)
//!
//! Every `evaluate()` call grades the response on two axes:
//! usefulness and honesty-about-limits.
//!
//! ```text
//!  useful?  honest-about-limits?  grade                  CI result
//!  ───────  ────────────────────  ────────────────────── ─────────
//!  yes      complete              UsefulComprehensive    pass
//!  yes      incomplete            UsefulIncomplete       pass
//!  no       flagged explicitly    IncompleteFlagged      pass
//!  no       confident/guessing    WrongConfident         HARD FAIL
//! ```
//!
//! `WrongConfident` is the only hard fail: the agent made definitive
//! claims it cannot support from the transcript (or speculated about
//! intent/psychology rather than citing observable behaviour).
//!
//! ## Behavioral-evidence authoring contract (GET-199)
//!
//! Set `must_not_speculate: true` in a fixture's expectations to
//! enforce the contract: if the response contains phrases that
//! attribute intent, emotion, or psychology to a participant the
//! outcome is immediately `WrongConfident`. The rule is Granola's
//! prompt-craft lesson B09: "if you can point to it in the text it's
//! fair game; if you have to guess why, drop it."

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AttuneError, Result};

// ---------------------------------------------------------------------------
// Four-quadrant grade (GET-199)
// ---------------------------------------------------------------------------

/// Outcome grade on the useful × honest-about-limits rubric.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvalGrade {
    /// Agent reported what happened accurately and completely. Pass.
    UsefulComprehensive,
    /// Agent got the key points right but missed some details. Pass.
    UsefulIncomplete,
    /// Agent acknowledged its gaps explicitly. Pass.
    IncompleteFlagged,
    /// Agent made confident claims not grounded in the transcript, or
    /// speculated about intent / psychology. Hard CI fail.
    WrongConfident,
}

impl EvalGrade {
    pub fn is_hard_fail(&self) -> bool {
        matches!(self, EvalGrade::WrongConfident)
    }

    pub fn label(&self) -> &'static str {
        match self {
            EvalGrade::UsefulComprehensive => "useful+comprehensive",
            EvalGrade::UsefulIncomplete => "useful+incomplete",
            EvalGrade::IncompleteFlagged => "incomplete-flagged",
            EvalGrade::WrongConfident => "wrong-confident [HARD FAIL]",
        }
    }
}

// Phrases that signal the agent is guessing at intent or psychology
// rather than citing observable behaviour from the transcript. A single
// match flags the response as WrongConfident when must_not_speculate is on.
const SPECULATION_PHRASES: &[&str] = &[
    "seems to feel",
    "seems to think",
    "seems to want",
    "seems frustrated",
    "seems excited",
    "seems uncomfortable",
    "seems to be",
    "appears to feel",
    "appears to want",
    "appears frustrated",
    "appears to be struggling",
    "appears to be excited",
    "clearly intends",
    "clearly feels",
    "clearly wants",
    "clearly thinks",
    "is trying to",
    "is hoping to",
    "is worried about",
    "is frustrated",
    "is excited about",
    "felt like",
    "feel like",
    "intention was",
    "intent was",
    "underlying concern",
    "real concern",
    "deeper issue",
    "reading between the lines",
    "tone suggests",
    "tone implies",
    "body language",
    "emotionally",
    "psychological",
];

// Phrases that signal the agent is being transparent about gaps.
// Presence of any of these in a failing response upgrades it from
// WrongConfident to IncompleteFlagged.
const HEDGING_PHRASES: &[&str] = &[
    "couldn't find",
    "could not find",
    "not enough information",
    "insufficient information",
    "i don't see",
    "i cannot see",
    "unclear from the transcript",
    "not clear from the transcript",
    "unclear in the transcript",
    "no mention of",
    "not mentioned",
    "not discussed",
    "the transcript doesn't",
    "the transcript does not",
    "limited information",
    "transcript is brief",
    "transcript was brief",
    "based on what's available",
    "based on available information",
    "no explicit",
    "not explicitly",
    "none found",
    "none.",
    "none\n",
];

// ---------------------------------------------------------------------------
// Fixture types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvalFixture {
    pub id: String,
    pub agent_id: String,
    pub transcript: String,
    #[serde(default)]
    pub expectations: EvalExpectations,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EvalExpectations {
    /// Substrings that must appear in the agent's response.
    #[serde(default)]
    pub must_contain: Vec<String>,
    /// Substrings that must NOT appear (catches "I cannot do that"
    /// refusals and other failure modes).
    #[serde(default)]
    pub must_not_contain: Vec<String>,
    /// Lower-bound word count on the response. None / 0 disables.
    #[serde(default)]
    pub min_words: Option<usize>,
    /// Upper-bound word count on the response.
    #[serde(default)]
    pub max_words: Option<usize>,
    /// Behavioral-evidence contract (GET-199). When true, the grader
    /// scans for psychological-speculation phrases. Any match upgrades
    /// the grade to WrongConfident regardless of other checks.
    #[serde(default)]
    pub must_not_speculate: bool,
}

#[derive(Debug, Clone)]
pub struct EvalOutcome {
    pub fixture_id: String,
    pub passed: bool,
    pub failures: Vec<String>,
    /// Four-quadrant grade on the useful × honest-about-limits rubric.
    pub grade: EvalGrade,
}

/// Load every `.json` fixture under `dir`. Files that fail to parse
/// are skipped with a tracing::warn so a typo in one fixture doesn't
/// invalidate the rest of the suite.
pub fn load_fixtures(dir: &Path) -> Vec<EvalFixture> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|raw| serde_json::from_str::<EvalFixture>(&raw).map_err(|e| e.to_string()))
        {
            Ok(f) => out.push(f),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "skipping unreadable eval fixture");
            }
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Apply the fixture's expectations to a model response and return a
/// graded outcome with human-readable failure reasons. Pure function —
/// call sites supply the response however they obtained it (live API,
/// recorded tape, stub provider).
///
/// Grading order (GET-199):
/// 1. Behavioral contract: `must_not_speculate` → WrongConfident on any
///    speculation phrase (hard fail regardless of other checks).
/// 2. Content checks: `must_contain` / `must_not_contain` / word counts.
/// 3. Grade assignment on useful × honest-about-limits:
///    - all checks pass → UsefulComprehensive
///    - only content failures + response hedges → IncompleteFlagged
///    - only content failures + no hedging → WrongConfident (hard fail)
///    - non-content failure (word count etc.) → UsefulIncomplete
pub fn evaluate(fixture: &EvalFixture, response: &str) -> EvalOutcome {
    let mut failures = Vec::new();
    let lower = response.to_lowercase();

    // --- Behavioral-evidence contract (GET-199) ---
    let mut speculation_hit: Option<String> = None;
    if fixture.expectations.must_not_speculate {
        for phrase in SPECULATION_PHRASES {
            if lower.contains(phrase) {
                speculation_hit = Some((*phrase).to_string());
                failures.push(format!(
                    "behavioral contract violated — psychological speculation detected: {phrase:?}"
                ));
                break;
            }
        }
    }

    // --- Content checks ---
    let mut content_failures: Vec<String> = Vec::new();
    for needle in &fixture.expectations.must_contain {
        if !lower.contains(&needle.to_lowercase()) {
            content_failures.push(format!("response missing required substring: {needle:?}"));
        }
    }
    for needle in &fixture.expectations.must_not_contain {
        if lower.contains(&needle.to_lowercase()) {
            content_failures.push(format!("response contains banned substring: {needle:?}"));
        }
    }
    failures.extend(content_failures.iter().cloned());

    let words = response.split_whitespace().count();
    let mut structural_failures: Vec<String> = Vec::new();
    if let Some(min) = fixture.expectations.min_words {
        if min > 0 && words < min {
            structural_failures.push(format!("response is {words} words, expected >= {min}"));
        }
    }
    if let Some(max) = fixture.expectations.max_words {
        if words > max {
            structural_failures.push(format!("response is {words} words, expected <= {max}"));
        }
    }
    failures.extend(structural_failures.iter().cloned());

    // --- Four-quadrant grade assignment ---
    let grade = if speculation_hit.is_some() {
        // Behavioral contract breach → always WrongConfident.
        EvalGrade::WrongConfident
    } else if failures.is_empty() {
        EvalGrade::UsefulComprehensive
    } else if !structural_failures.is_empty() && content_failures.is_empty() {
        // Only structural (word count) issues — the content is fine.
        EvalGrade::UsefulIncomplete
    } else if !content_failures.is_empty() {
        // Content failures: does the agent admit the gap?
        let hedges = HEDGING_PHRASES.iter().any(|p| lower.contains(p));
        if hedges {
            EvalGrade::IncompleteFlagged
        } else {
            EvalGrade::WrongConfident
        }
    } else {
        EvalGrade::UsefulIncomplete
    };

    let passed = failures.is_empty() || grade == EvalGrade::IncompleteFlagged;
    EvalOutcome {
        fixture_id: fixture.id.clone(),
        passed,
        failures,
        grade,
    }
}

/// Validate a fixture's agent_id is a known default agent. Used by
/// the cargo-test sanity sweep so adding a fixture with a typo'd
/// agent name fails CI immediately instead of mysteriously at run
/// time. Returns the resolved agent for chaining.
pub fn assert_known_agent(fixture: &EvalFixture) -> Result<crate::llm::Agent> {
    crate::llm::agents::by_id(&fixture.agent_id).ok_or_else(|| {
        AttuneError::Storage(format!(
            "eval fixture {} references unknown agent_id {}",
            fixture.id, fixture.agent_id
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sample() -> EvalFixture {
        EvalFixture {
            id: "summarize-smoke".to_string(),
            agent_id: "summarize".to_string(),
            transcript: "alice: hi\nbob: hello".to_string(),
            expectations: EvalExpectations {
                must_contain: vec!["alice".to_string()],
                must_not_contain: vec!["I cannot".to_string()],
                min_words: Some(3),
                max_words: Some(100),
                must_not_speculate: false,
            },
            notes: None,
        }
    }

    // ------------------------------------------------------------------
    // Existing checks
    // ------------------------------------------------------------------

    #[test]
    fn evaluate_passes_when_all_expectations_hold() {
        let f = sample();
        let r = evaluate(&f, "Alice greeted Bob in the meeting.");
        assert!(r.passed, "expected pass, got failures: {:?}", r.failures);
        assert_eq!(r.grade, EvalGrade::UsefulComprehensive);
    }

    #[test]
    fn evaluate_flags_missing_required_substring() {
        let mut f = sample();
        f.expectations.must_contain.push("pricing".into());
        let r = evaluate(&f, "Alice greeted Bob in the meeting.");
        assert!(!r.passed);
        assert!(r.failures.iter().any(|m| m.contains("pricing")));
    }

    #[test]
    fn evaluate_flags_banned_substring() {
        let f = sample();
        let r = evaluate(&f, "alice — I cannot answer that question");
        assert!(!r.passed);
        assert!(r.failures.iter().any(|m| m.contains("banned")));
    }

    #[test]
    fn evaluate_enforces_word_count_bounds() {
        let mut f = sample();
        f.expectations.min_words = Some(20);
        let r = evaluate(&f, "alice short reply.");
        assert!(!r.passed);
        assert!(r.failures.iter().any(|m| m.contains("words")));
    }

    // ------------------------------------------------------------------
    // Four-quadrant grading (GET-199)
    // ------------------------------------------------------------------

    #[test]
    fn grade_useful_comprehensive_on_full_pass() {
        let f = sample();
        let r = evaluate(&f, "Alice greeted Bob in the meeting.");
        assert_eq!(r.grade, EvalGrade::UsefulComprehensive);
        assert!(!r.grade.is_hard_fail());
    }

    #[test]
    fn grade_useful_incomplete_on_word_count_only() {
        let mut f = sample();
        f.expectations.min_words = Some(50);
        let r = evaluate(&f, "Alice greeted Bob in the meeting.");
        assert_eq!(r.grade, EvalGrade::UsefulIncomplete);
        assert!(!r.grade.is_hard_fail());
    }

    #[test]
    fn grade_incomplete_flagged_when_content_missing_but_hedged() {
        let mut f = sample();
        f.expectations.must_contain.push("action items".into());
        let r = evaluate(
            &f,
            "Alice was there. No mention of action items in the transcript.",
        );
        // "alice" is present; "action items" appears verbatim → must_contain passes.
        // Change: use a term that is genuinely absent to force IncompleteFlagged.
        let mut f2 = sample();
        f2.expectations
            .must_contain
            .push("quarterly roadmap".into());
        let r2 = evaluate(
            &f2,
            "Alice was there. No explicit mention of that topic could not be found in this transcript.",
        );
        assert_eq!(r2.grade, EvalGrade::IncompleteFlagged);
        assert!(!r2.grade.is_hard_fail());
        assert!(r2.passed, "IncompleteFlagged should count as passed");
        // Confirm the first scenario passes normally.
        assert!(r.passed);
    }

    #[test]
    fn grade_wrong_confident_when_content_missing_no_hedge() {
        let mut f = sample();
        f.expectations.must_contain.push("pricing".into());
        let r = evaluate(&f, "Alice greeted Bob.");
        assert_eq!(r.grade, EvalGrade::WrongConfident);
        assert!(r.grade.is_hard_fail());
        assert!(!r.passed);
    }

    // ------------------------------------------------------------------
    // Behavioral-evidence contract (GET-199)
    // ------------------------------------------------------------------

    #[test]
    fn speculation_flag_off_ignores_speculation_phrases() {
        let f = sample();
        let r = evaluate(&f, "Alice seems to feel good about the meeting.");
        // speculation check is off — should still grade normally
        assert!(!r.failures.iter().any(|m| m.contains("behavioral contract")));
    }

    #[test]
    fn speculation_flag_on_catches_intent_guessing() {
        let mut f = sample();
        f.expectations.must_not_speculate = true;
        let r = evaluate(
            &f,
            "Alice seems frustrated with Bob's approach in the meeting.",
        );
        assert!(r.grade.is_hard_fail());
        assert_eq!(r.grade, EvalGrade::WrongConfident);
        assert!(r.failures.iter().any(|m| m.contains("behavioral contract")));
    }

    #[test]
    fn speculation_flag_on_passes_observable_claims() {
        let mut f = sample();
        f.expectations.must_not_speculate = true;
        let r = evaluate(&f, "Alice said hello and Bob replied.");
        assert!(r.passed, "observable claim should pass: {:?}", r.failures);
        assert!(!r.grade.is_hard_fail());
    }

    #[test]
    fn speculation_flag_on_catches_clearly_intends() {
        let mut f = sample();
        f.expectations.must_not_speculate = true;
        let r = evaluate(&f, "alice clearly intends to follow up next week.");
        assert!(r.grade.is_hard_fail());
    }

    #[test]
    fn grade_label_strings_are_stable() {
        assert_eq!(
            EvalGrade::UsefulComprehensive.label(),
            "useful+comprehensive"
        );
        assert!(EvalGrade::WrongConfident.label().contains("HARD FAIL"));
    }

    // ------------------------------------------------------------------
    // Fixture loading
    // ------------------------------------------------------------------

    #[test]
    fn load_fixtures_skips_unparsable_files() {
        let dir = tempfile::tempdir().unwrap();
        let good = serde_json::to_string(&sample()).unwrap();
        fs::write(dir.path().join("good.json"), good).unwrap();
        fs::write(dir.path().join("bad.json"), "{not-json").unwrap();
        fs::write(dir.path().join("ignored.txt"), "skip me").unwrap();
        let loaded = load_fixtures(dir.path());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "summarize-smoke");
    }

    #[test]
    fn shipped_fixtures_reference_known_agents() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("evals");
        if !dir.is_dir() {
            return;
        }
        let fixtures = load_fixtures(&dir);
        for f in fixtures {
            assert_known_agent(&f)
                .unwrap_or_else(|e| panic!("fixture {} references unknown agent: {e}", f.id));
        }
    }
}
