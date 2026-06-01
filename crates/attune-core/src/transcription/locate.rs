//! Locate an evidence span inside a transcript. v2 finding 038 / GET-41.
//!
//! Every memory, task, and decision Attune emits carries a verbatim
//! transcript span (see #031 / GET-57). To make those backlinks
//! actually clickable — "jump to that exact second of audio" — we
//! need to find which segment the span lives in. This module is the
//! pure helper.
//!
//! Algorithm:
//!   * Normalise (collapse whitespace, lowercase) both the haystack
//!     (the channel's joined segment text) and the needle (the span).
//!   * Find the byte index of the first match.
//!   * Walk the segments forward, accumulating their normalised text
//!     lengths, until the cumulative byte index passes the match. That
//!     segment is the hit.
//!   * Return the segment index, the original `start_seconds` /
//!     `end_seconds`, and the channel id ("mic" / "system").
//!
//! Per-channel pass: we try each channel in order and return the first
//! hit. The two channels rarely contain the same speech, so collisions
//! are not a practical concern.

use super::SessionTranscript;
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct TranscriptHit {
    pub channel: String,
    pub segment_index: usize,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub matched_text: String,
}

/// Find which channel + segment the span lives in. Returns None if
/// the span does not appear in the transcript.
pub fn locate_span(transcript: &SessionTranscript, span: &str) -> Option<TranscriptHit> {
    let needle = normalize(span);
    if needle.is_empty() {
        return None;
    }
    for channel in &transcript.channels {
        // Build a cumulative-length table over normalised segment text
        // joined with single spaces. The byte index of the first
        // separator after segment i is `cum[i+1] - 1`.
        let mut cum: Vec<usize> = Vec::with_capacity(channel.segments.len() + 1);
        cum.push(0);
        let mut joined = String::new();
        for (i, seg) in channel.segments.iter().enumerate() {
            let n = normalize(&seg.text);
            if i > 0 && !joined.is_empty() && !n.is_empty() {
                joined.push(' ');
            }
            joined.push_str(&n);
            cum.push(joined.len());
        }
        if let Some(byte_idx) = joined.find(&needle) {
            // Walk cum[] to find the segment whose [cum[i]..cum[i+1])
            // range contains byte_idx.
            let seg_idx = match cum.binary_search_by(|probe| {
                if *probe <= byte_idx {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            }) {
                Ok(i) => i.saturating_sub(1),
                Err(i) => i.saturating_sub(1),
            };
            let seg = &channel.segments[seg_idx];
            return Some(TranscriptHit {
                channel: channel.channel.clone(),
                segment_index: seg_idx,
                start_seconds: seg.start_seconds,
                end_seconds: seg.end_seconds,
                matched_text: seg.text.clone(),
            });
        }
    }
    None
}

/// Best-effort fuzzy locate (GET-198): find the transcript segment that
/// most overlaps a *paraphrased* query line — an enhanced-note line the AI
/// wrote, which won't appear verbatim. Scores each segment by shared
/// content words (≥4 chars, so short stopwords drop out across languages)
/// and returns the best, or `None` when nothing clears the confidence
/// floor — so the UI shows no "jump" for a line it can't pin to a moment.
pub fn locate_fuzzy(transcript: &SessionTranscript, query: &str) -> Option<TranscriptHit> {
    use std::collections::HashSet;

    let q_tokens = content_tokens(query);
    let q_set: HashSet<&str> = q_tokens.iter().map(String::as_str).collect();
    if q_set.len() < 2 {
        return None;
    }

    /// At least this fraction of the line's content words must appear in a
    /// single segment for it to count as the source.
    const FLOOR: f32 = 0.34;

    let mut best_score = 0.0_f32;
    let mut best_hit: Option<TranscriptHit> = None;
    for channel in &transcript.channels {
        for (i, seg) in channel.segments.iter().enumerate() {
            let s_tokens = content_tokens(&seg.text);
            let s_set: HashSet<&str> = s_tokens.iter().map(String::as_str).collect();
            if s_set.is_empty() {
                continue;
            }
            let shared = q_set.iter().filter(|t| s_set.contains(*t)).count();
            if shared < 2 {
                continue;
            }
            let score = shared as f32 / q_set.len() as f32;
            if score > best_score {
                best_score = score;
                best_hit = Some(TranscriptHit {
                    channel: channel.channel.clone(),
                    segment_index: i,
                    start_seconds: seg.start_seconds,
                    end_seconds: seg.end_seconds,
                    matched_text: seg.text.clone(),
                });
            }
        }
    }
    if best_score >= FLOOR {
        best_hit
    } else {
        None
    }
}

/// How many distinct transcript segments corroborate a claim — segments
/// sharing ≥2 content words with it (GET-209). A claim whose evidence shows
/// up in only ONE segment rests on a single passing remark; the agents flag
/// it "mentioned once" so a forgotten throwaway line never ambushes the
/// user. Returns 0 when the claim is too short to judge (so callers don't
/// flag it).
pub fn support_count(transcript: &SessionTranscript, claim: &str) -> usize {
    use std::collections::HashSet;

    let c_tokens = content_tokens(claim);
    let c_set: HashSet<&str> = c_tokens.iter().map(String::as_str).collect();
    if c_set.len() < 2 {
        return 0;
    }
    let mut count = 0;
    for channel in &transcript.channels {
        for seg in &channel.segments {
            let s_tokens = content_tokens(&seg.text);
            let s_set: HashSet<&str> = s_tokens.iter().map(String::as_str).collect();
            let shared = c_set.iter().filter(|t| s_set.contains(*t)).count();
            if shared >= 2 {
                count += 1;
            }
        }
    }
    count
}

/// Lowercased alphanumeric tokens of length ≥ 4 — the content-bearing
/// words. The length floor naturally drops most stopwords without a
/// language-specific list (the transcripts are multilingual).
fn content_tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.chars().count() >= 4)
        .map(|t| t.to_string())
        .collect()
}

fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = true;
    for ch in s.chars() {
        let lc = ch.to_ascii_lowercase();
        if lc.is_whitespace() {
            if !last_space {
                out.push(' ');
                last_space = true;
            }
        } else {
            out.push(lc);
            last_space = false;
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
    use crate::transcription::{ChannelTranscript, TranscriptSegment};

    fn seg(t: &str, start: f64, end: f64) -> TranscriptSegment {
        TranscriptSegment {
            start_seconds: start,
            end_seconds: end,
            text: t.to_string(),
            speaker: None,
            language: None,
        }
    }

    fn fixture() -> SessionTranscript {
        SessionTranscript {
            channels: vec![
                ChannelTranscript {
                    channel: "mic".into(),
                    language: Some("en".into()),
                    segments: vec![
                        seg("Hi everyone, welcome to the meeting.", 0.0, 3.5),
                        seg("Let's ship the redesign by Friday.", 3.5, 7.0),
                        seg("Alice will handle the press release.", 7.0, 10.0),
                    ],
                },
                ChannelTranscript {
                    channel: "system".into(),
                    language: Some("en".into()),
                    segments: vec![seg("Sounds great to me.", 6.5, 8.0)],
                },
            ],
        }
    }

    #[test]
    fn fuzzy_locates_paraphrased_line_to_best_segment() {
        let t = fixture();
        // A paraphrase of segment 1 — no verbatim substring, but shares
        // the content words "ship"/"redesign"/"Friday".
        let hit = locate_fuzzy(&t, "Team agreed to ship the redesign before Friday").unwrap();
        assert_eq!(hit.segment_index, 1);
        assert!((hit.start_seconds - 3.5).abs() < 1e-6);
    }

    #[test]
    fn fuzzy_returns_none_for_unrelated_line() {
        let t = fixture();
        // Nothing in the transcript is about budget/quarterly numbers.
        assert!(locate_fuzzy(&t, "Quarterly budget projections exceeded estimates").is_none());
    }

    #[test]
    fn fuzzy_needs_at_least_two_content_words() {
        let t = fixture();
        // One content word ("redesign") isn't enough signal to jump.
        assert!(locate_fuzzy(&t, "the redesign").is_none());
    }

    #[test]
    fn support_count_flags_single_utterance_vs_corroborated() {
        let t = SessionTranscript {
            channels: vec![ChannelTranscript {
                channel: "system".into(),
                language: Some("en".into()),
                segments: vec![
                    seg("We should ship the redesign before the launch.", 0.0, 3.0),
                    seg(
                        "The redesign and the launch are our priorities.",
                        30.0,
                        33.0,
                    ),
                    seg("Anyway I once skydived over Dubai years ago.", 60.0, 63.0),
                ],
            }],
        };
        // "redesign" + "launch" recur across two segments → corroborated.
        assert_eq!(support_count(&t, "ship the redesign before launch"), 2);
        // The skydiving aside shares ≥2 content words with exactly one
        // segment → mentioned once.
        assert_eq!(support_count(&t, "skydived over Dubai"), 1);
        // Too few content words to judge.
        assert_eq!(support_count(&t, "Dubai"), 0);
    }

    #[test]
    fn finds_span_in_second_segment_with_correct_timestamps() {
        let t = fixture();
        let hit = locate_span(&t, "ship the redesign by Friday").unwrap();
        assert_eq!(hit.channel, "mic");
        assert_eq!(hit.segment_index, 1);
        assert!((hit.start_seconds - 3.5).abs() < 1e-6);
        assert!((hit.end_seconds - 7.0).abs() < 1e-6);
    }

    #[test]
    fn finds_span_on_second_channel() {
        let t = fixture();
        let hit = locate_span(&t, "Sounds great to me").unwrap();
        assert_eq!(hit.channel, "system");
        assert_eq!(hit.segment_index, 0);
    }

    #[test]
    fn span_spanning_segment_boundary_maps_to_first_segment() {
        // Span crosses segments 1 → 2; the matched start lives in
        // segment 1 so that's where we point the user.
        let t = fixture();
        let hit = locate_span(&t, "by Friday. Alice will").unwrap();
        assert_eq!(hit.segment_index, 1);
    }

    #[test]
    fn returns_none_when_span_absent() {
        let t = fixture();
        assert!(locate_span(&t, "launch on Mars").is_none());
        assert!(locate_span(&t, "").is_none());
    }

    #[test]
    fn whitespace_and_case_collapse() {
        let t = fixture();
        let hit = locate_span(&t, "SHIP   the\n redesign").unwrap();
        assert_eq!(hit.segment_index, 1);
    }
}
