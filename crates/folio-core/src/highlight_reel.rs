use serde::{Deserialize, Serialize};

use crate::transcription::SessionTranscript;

pub const REEL_MIN_SECONDS: f64 = 60.0;
pub const REEL_MAX_SECONDS: f64 = 90.0;
pub const MIN_GAP_SECONDS: f64 = 1.5;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReelCut {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub subtitle: String,
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReelPlan {
    pub total_seconds: f64,
    pub cuts: Vec<ReelCut>,
}

const DECISION_MARKERS: &[&str] = &[
    "decided",
    "we agreed",
    "we'll",
    "we will",
    "let's",
    "going to ship",
    "ship by",
    "decision",
    "blocker",
    "owner",
    "action item",
];

pub fn score_segment(text: &str) -> u32 {
    let lc = text.to_lowercase();
    let marker_hits = DECISION_MARKERS.iter().filter(|m| lc.contains(*m)).count() as u32;
    if marker_hits == 0 {
        return 0;
    }
    let word_count = text.split_whitespace().count() as u32;
    marker_hits * 50 + word_count.min(40)
}

pub fn plan(transcript: &SessionTranscript) -> Option<ReelPlan> {
    let mut scored: Vec<(u32, ReelCut)> = Vec::new();
    for channel in &transcript.channels {
        for seg in &channel.segments {
            let score = score_segment(&seg.text);
            if score == 0 {
                continue;
            }
            scored.push((
                score,
                ReelCut {
                    start_seconds: seg.start_seconds,
                    end_seconds: seg.end_seconds,
                    subtitle: seg.text.trim().to_string(),
                    channel: channel.channel.clone(),
                },
            ));
        }
    }
    if scored.is_empty() {
        return None;
    }
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0).then(
            a.1.start_seconds
                .partial_cmp(&b.1.start_seconds)
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });

    let mut picked: Vec<ReelCut> = Vec::new();
    let mut total = 0.0_f64;
    for (_, candidate) in scored {
        if total >= REEL_MAX_SECONDS {
            break;
        }
        if !picked.iter().all(|c| separated_enough(c, &candidate)) {
            continue;
        }
        let dur = (candidate.end_seconds - candidate.start_seconds).max(0.0);
        if total + dur > REEL_MAX_SECONDS {
            continue;
        }
        total += dur;
        picked.push(candidate);
        if total >= REEL_MIN_SECONDS {
            break;
        }
    }
    if picked.is_empty() {
        return None;
    }
    picked.sort_by(|a, b| {
        a.start_seconds
            .partial_cmp(&b.start_seconds)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Some(ReelPlan {
        total_seconds: total,
        cuts: picked,
    })
}

fn separated_enough(a: &ReelCut, b: &ReelCut) -> bool {
    if a.channel != b.channel {
        return true;
    }
    let lo = a.start_seconds.min(b.start_seconds);
    let hi = a.end_seconds.max(b.end_seconds);
    let overlap = a.end_seconds.min(b.end_seconds) - a.start_seconds.max(b.start_seconds);
    if overlap > 0.0 {
        return false;
    }
    (hi - lo) - (a.end_seconds - a.start_seconds) - (b.end_seconds - b.start_seconds)
        >= MIN_GAP_SECONDS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::{ChannelTranscript, TranscriptSegment};

    fn seg(text: &str, start: f64, end: f64) -> TranscriptSegment {
        TranscriptSegment {
            start_seconds: start,
            end_seconds: end,
            text: text.into(),
            speaker: None,
            language: None,
        }
    }

    fn fixture() -> SessionTranscript {
        SessionTranscript {
            channels: vec![ChannelTranscript {
                channel: "mic".into(),
                language: None,
                segments: vec![
                    seg("Hello and welcome to the meeting.", 0.0, 5.0),
                    seg(
                        "We agreed to ship the redesign by Friday, that is the decision.",
                        20.0,
                        35.0,
                    ),
                    seg(
                        "Alice will own the announcement, that's the action item.",
                        40.0,
                        55.0,
                    ),
                    seg(
                        "Bob has a blocker on the legal review, we'll chase it tomorrow.",
                        60.0,
                        78.0,
                    ),
                    seg("Thanks everyone, goodbye.", 80.0, 84.0),
                ],
            }],
        }
    }

    #[test]
    fn score_segment_rewards_decision_markers() {
        let plain = score_segment("today is sunny");
        let decision = score_segment("we agreed to ship the redesign");
        assert!(decision > plain);
    }

    #[test]
    fn score_segment_returns_zero_on_empty_text() {
        assert_eq!(score_segment(""), 0);
    }

    #[test]
    fn plan_returns_none_for_pure_smalltalk() {
        let smalltalk = SessionTranscript {
            channels: vec![ChannelTranscript {
                channel: "mic".into(),
                language: None,
                segments: vec![seg("Hi there.", 0.0, 2.0), seg("Bye.", 2.0, 3.0)],
            }],
        };
        assert!(plan(&smalltalk).is_none());
    }

    #[test]
    fn plan_picks_decision_dense_segments() {
        let p = plan(&fixture()).unwrap();
        assert!(p.total_seconds >= 15.0);
        assert!(p.cuts.iter().any(|c| c.subtitle.contains("redesign")));
        assert!(p.cuts.iter().any(|c| c.subtitle.contains("Alice")));
    }

    #[test]
    fn plan_keeps_total_under_reel_max() {
        let p = plan(&fixture()).unwrap();
        assert!(p.total_seconds <= REEL_MAX_SECONDS);
    }

    #[test]
    fn plan_emits_cuts_in_chronological_order() {
        let p = plan(&fixture()).unwrap();
        for window in p.cuts.windows(2) {
            assert!(window[0].start_seconds <= window[1].start_seconds);
        }
    }
}
