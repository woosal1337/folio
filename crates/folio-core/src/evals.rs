use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{FolioError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvalGrade {
    UsefulComprehensive,

    UsefulIncomplete,

    IncompleteFlagged,

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
    #[serde(default)]
    pub must_contain: Vec<String>,

    #[serde(default)]
    pub must_not_contain: Vec<String>,

    #[serde(default)]
    pub min_words: Option<usize>,

    #[serde(default)]
    pub max_words: Option<usize>,

    #[serde(default)]
    pub must_not_speculate: bool,
}

#[derive(Debug, Clone)]
pub struct EvalOutcome {
    pub fixture_id: String,
    pub passed: bool,
    pub failures: Vec<String>,

    pub grade: EvalGrade,
}

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

pub fn evaluate(fixture: &EvalFixture, response: &str) -> EvalOutcome {
    let mut failures = Vec::new();
    let lower = response.to_lowercase();

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

    let grade = if speculation_hit.is_some() {
        EvalGrade::WrongConfident
    } else if failures.is_empty() {
        EvalGrade::UsefulComprehensive
    } else if !structural_failures.is_empty() && content_failures.is_empty() {
        EvalGrade::UsefulIncomplete
    } else if !content_failures.is_empty() {
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

pub fn assert_known_agent(fixture: &EvalFixture) -> Result<crate::llm::Agent> {
    crate::llm::agents::by_id(&fixture.agent_id).ok_or_else(|| {
        FolioError::Storage(format!(
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

    #[test]
    fn speculation_flag_off_ignores_speculation_phrases() {
        let f = sample();
        let r = evaluate(&f, "Alice seems to feel good about the meeting.");

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
