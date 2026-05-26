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
            let seg_idx = match cum
                .binary_search_by(|probe| {
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
