use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const DEFAULT_MIN_CONFIDENCE: f32 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Verified,

    LowConfidence,

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
        let g = judge(0.55, "ship the redesign", TRANSCRIPT, 0.5);
        assert_eq!(g.verdict, Verdict::Verified);

        let g = judge(0.55, "ship the redesign", TRANSCRIPT, 0.8);
        assert_eq!(g.verdict, Verdict::LowConfidence);
    }
}
