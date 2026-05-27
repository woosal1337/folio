//! Confidence scoring + hallucination guard for extraction agents.
//! v2 finding 031 / GET-57.
//!
//! Every item produced by `extract-tasks`, `extract-memories`, and
//! `find-decisions` carries (a) a model-self-reported confidence score
//! in [0.0, 1.0] and (b) a verbatim transcript span the item is
//! grounded in. The guard here verifies the span actually appears in
//! the transcript (loose match: collapsed whitespace, case-insensitive)
//! and downgrades items that fail to "unverified".
//!
//! Items with `confidence < threshold` OR a missing evidence span are
//! marked `verified = false`. The caller surfaces them with an
//! "unverified" badge instead of silently dropping them — users see
//! every guess and decide whether to keep it.
//!
//! The default threshold is 0.6; this matches the value already in
//! the extract-memories prompt.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const DEFAULT_MIN_CONFIDENCE: f32 = 0.6;

/// Verdict the guard emits per item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Confidence >= threshold AND evidence span found verbatim in the
    /// transcript. The frontend renders these as normal items.
    Verified,
    /// Confidence under threshold but evidence span is present. The
    /// model is hedging; the UI surfaces an "unverified" badge.
    LowConfidence,
    /// Evidence span is empty or could not be located in the transcript.
    /// Most likely a hallucination — UI ships the "unverified" badge
    /// with a "no source" tooltip.
    NotGrounded,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct GuardedItem {
    pub confidence: f32,
    pub evidence_span: String,
    pub verdict: Verdict,
    pub verified: bool,
}

/// Build a GuardedItem from a confidence + evidence span + the full
/// transcript text. Pure function, no IO — the prompts feed us the
/// confidence and span; we judge.
pub fn judge(
    confidence: f32,
    evidence_span: &str,
    transcript: &str,
    threshold: f32,
) -> GuardedItem {
    let span = evidence_span.trim();
    let grounded = !span.is_empty() && contains_loose(transcript, span);
    let verdict = if !grounded {
        Verdict::NotGrounded
    } else if confidence < threshold {
        Verdict::LowConfidence
    } else {
        Verdict::Verified
    };
    GuardedItem {
        confidence,
        evidence_span: span.to_string(),
        verdict,
        verified: matches!(verdict, Verdict::Verified),
    }
}

/// Case-insensitive, collapsed-whitespace substring match. The
/// transcript has segment boundaries and line-wrapping that mangle a
/// naive `transcript.contains(span)`; collapsing runs of whitespace
/// to a single space on both sides recovers most legitimate spans
/// without going as far as fuzzy matching (which would re-introduce
/// hallucinations).
pub fn contains_loose(haystack: &str, needle: &str) -> bool {
    let h = normalize(haystack);
    let n = normalize(needle);
    !n.is_empty() && h.contains(&n)
}

fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_was_space = true;
    for ch in s.chars() {
        let lc = ch.to_ascii_lowercase();
        if lc.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        } else {
            out.push(lc);
            last_was_space = false;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRANSCRIPT: &str =
        "Alice: we should ship the redesign \nby Friday.\n\nBob: I'll handle the press release.";

    #[test]
    fn verified_when_high_confidence_and_grounded() {
        let g = judge(
            0.9,
            "ship the redesign by Friday",
            TRANSCRIPT,
            DEFAULT_MIN_CONFIDENCE,
        );
        assert_eq!(g.verdict, Verdict::Verified);
        assert!(g.verified);
    }

    #[test]
    fn low_confidence_when_under_threshold_but_grounded() {
        let g = judge(
            0.4,
            "I'll handle the press release",
            TRANSCRIPT,
            DEFAULT_MIN_CONFIDENCE,
        );
        assert_eq!(g.verdict, Verdict::LowConfidence);
        assert!(!g.verified);
    }

    #[test]
    fn not_grounded_when_span_missing() {
        let g = judge(
            0.95,
            "let's launch on Mars",
            TRANSCRIPT,
            DEFAULT_MIN_CONFIDENCE,
        );
        assert_eq!(g.verdict, Verdict::NotGrounded);
        assert!(!g.verified);
    }

    #[test]
    fn not_grounded_when_span_empty() {
        let g = judge(0.99, "", TRANSCRIPT, DEFAULT_MIN_CONFIDENCE);
        assert_eq!(g.verdict, Verdict::NotGrounded);
    }

    #[test]
    fn whitespace_and_case_collapse() {
        assert!(contains_loose("SHIP   the\n redesign", "ship the redesign"));
        assert!(!contains_loose("ship the redesign", "ship Mars"));
    }

    #[test]
    fn custom_threshold_respected() {
        // With threshold 0.5, confidence 0.55 is verified.
        let g = judge(0.55, "ship the redesign", TRANSCRIPT, 0.5);
        assert_eq!(g.verdict, Verdict::Verified);
        // With threshold 0.8, the same item is low-confidence.
        let g = judge(0.55, "ship the redesign", TRANSCRIPT, 0.8);
        assert_eq!(g.verdict, Verdict::LowConfidence);
    }
}
