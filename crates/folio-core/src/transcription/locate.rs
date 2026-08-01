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

pub fn locate_fuzzy(transcript: &SessionTranscript, query: &str) -> Option<TranscriptHit> {
    use std::collections::HashSet;

    let q_tokens = content_tokens(query);
    let q_set: HashSet<&str> = q_tokens.iter().map(String::as_str).collect();
    if q_set.len() < 2 {
        return None;
    }

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

fn content_tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.chars().count() >= 4)
        .map(|t| t.to_string())
        .collect()
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

        let hit = locate_fuzzy(&t, "Team agreed to ship the redesign before Friday").unwrap();
        assert_eq!(hit.segment_index, 1);
        assert!((hit.start_seconds - 3.5).abs() < 1e-6);
    }

    #[test]
    fn fuzzy_returns_none_for_unrelated_line() {
        let t = fixture();

        assert!(locate_fuzzy(&t, "Quarterly budget projections exceeded estimates").is_none());
    }

    #[test]
    fn fuzzy_needs_at_least_two_content_words() {
        let t = fixture();

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

        assert_eq!(support_count(&t, "ship the redesign before launch"), 2);

        assert_eq!(support_count(&t, "skydived over Dubai"), 1);

        assert_eq!(support_count(&t, "Dubai"), 0);
    }
}
