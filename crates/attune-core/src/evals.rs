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
//!     "max_words": 250
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

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AttuneError, Result};

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
}

#[derive(Debug, Clone)]
pub struct EvalOutcome {
    pub fixture_id: String,
    pub passed: bool,
    pub failures: Vec<String>,
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
/// pass/fail outcome with human-readable failure reasons. Pure
/// function — call sites supply the response however they obtained
/// it (live API, recorded tape, stub provider).
pub fn evaluate(fixture: &EvalFixture, response: &str) -> EvalOutcome {
    let mut failures = Vec::new();
    let lower = response.to_lowercase();
    for needle in &fixture.expectations.must_contain {
        if !lower.contains(&needle.to_lowercase()) {
            failures.push(format!("response missing required substring: {needle:?}"));
        }
    }
    for needle in &fixture.expectations.must_not_contain {
        if lower.contains(&needle.to_lowercase()) {
            failures.push(format!("response contains banned substring: {needle:?}"));
        }
    }
    let words = response.split_whitespace().count();
    if let Some(min) = fixture.expectations.min_words {
        if min > 0 && words < min {
            failures.push(format!("response is {words} words, expected >= {min}"));
        }
    }
    if let Some(max) = fixture.expectations.max_words {
        if words > max {
            failures.push(format!("response is {words} words, expected <= {max}"));
        }
    }
    EvalOutcome {
        fixture_id: fixture.id.clone(),
        passed: failures.is_empty(),
        failures,
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
            },
            notes: None,
        }
    }

    #[test]
    fn evaluate_passes_when_all_expectations_hold() {
        let f = sample();
        let r = evaluate(&f, "Alice greeted Bob in the meeting.");
        assert!(r.passed, "expected pass, got failures: {:?}", r.failures);
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
