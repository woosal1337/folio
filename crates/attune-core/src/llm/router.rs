use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::super::transcription::SessionTranscript;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub enum ModelTier {
    Standard,

    Premium,
}

impl ModelTier {
    pub fn openai_model_id(self) -> &'static str {
        match self {
            ModelTier::Standard => "gpt-4o-mini",
            ModelTier::Premium => "gpt-4o",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct RouterPolicy {
    pub voice_memo_max_secs: f64,

    pub summarise_min_words: usize,

    pub memories_min_words: usize,

    pub decisions_min_participants: usize,

    pub premium_tier_min_words: usize,
}

impl Default for RouterPolicy {
    fn default() -> Self {
        Self {
            voice_memo_max_secs: 120.0,
            summarise_min_words: 30,
            memories_min_words: 50,
            decisions_min_participants: 2,

            premium_tier_min_words: 5_000,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct TranscriptSignals {
    pub duration_secs: f64,
    pub word_count: usize,
    pub participants: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct RouterDecision {
    pub run_summarise: bool,
    pub run_extract_tasks: bool,
    pub run_extract_memories: bool,
    pub run_find_decisions: bool,
    pub run_autoname: bool,

    pub model_tier: ModelTier,
}

pub fn signals_from(transcript: &SessionTranscript) -> TranscriptSignals {
    let mut max_end = 0.0_f64;
    let mut word_count = 0usize;
    let mut participants = 0usize;
    for channel in &transcript.channels {
        let mut channel_has_speech = false;
        for seg in &channel.segments {
            if seg.end_seconds > max_end {
                max_end = seg.end_seconds;
            }
            let words = seg.text.split_whitespace().count();
            if words > 0 {
                channel_has_speech = true;
            }
            word_count += words;
        }
        if channel_has_speech {
            participants += 1;
        }
    }
    TranscriptSignals {
        duration_secs: max_end,
        word_count,
        participants,
    }
}

pub fn decide(signals: TranscriptSignals, policy: RouterPolicy) -> RouterDecision {
    let is_voice_memo = signals.duration_secs <= policy.voice_memo_max_secs;
    let too_short_to_summarise = signals.word_count < policy.summarise_min_words;
    let too_short_for_memories = signals.word_count < policy.memories_min_words;
    let solo_speaker = signals.participants < policy.decisions_min_participants;

    let model_tier = if signals.word_count >= policy.premium_tier_min_words {
        ModelTier::Premium
    } else {
        ModelTier::Standard
    };

    RouterDecision {
        run_summarise: !too_short_to_summarise,

        run_extract_tasks: !is_voice_memo,
        run_extract_memories: !too_short_for_memories,

        run_find_decisions: !solo_speaker && !is_voice_memo,

        run_autoname: true,
        model_tier,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::{ChannelTranscript, TranscriptSegment};

    fn seg(t: &str, start: f64, end: f64) -> TranscriptSegment {
        TranscriptSegment {
            start_seconds: start,
            end_seconds: end,
            text: t.into(),
            speaker: None,
            language: None,
        }
    }
    fn ch(name: &str, segs: Vec<TranscriptSegment>) -> ChannelTranscript {
        ChannelTranscript {
            channel: name.into(),
            language: None,
            segments: segs,
        }
    }

    #[test]
    fn voice_memo_skips_tasks_and_decisions_but_still_summarises_long_ones() {
        let t = SessionTranscript {
            channels: vec![ch(
                "mic",
                vec![seg(
                    "Reminder to self: pick up groceries, buy a new charger, and \
                     call mom on Sunday. Also remember to ship the redesign before \
                     Friday and double-check the colour palette with Alice and Bob \
                     on the design review thread.",
                    0.0,
                    60.0,
                )],
            )],
        };
        let signals = signals_from(&t);
        assert!(signals.duration_secs <= 120.0);
        let decision = decide(signals, RouterPolicy::default());
        assert!(decision.run_summarise);
        assert!(!decision.run_extract_tasks, "voice memo skips tasks");
        assert!(!decision.run_find_decisions, "voice memo skips decisions");
        assert!(decision.run_autoname);
    }

    #[test]
    fn long_two_party_meeting_runs_everything() {
        let body = "We agreed to ship the redesign by Friday. Alice will own \
            the press release. Bob raised concerns about the legal review \
            timeline and the contract renewal that comes up at the end of \
            next month. We also walked through the new pricing tier deck \
            and confirmed the launch announcement will go out on the same \
            day as the public website refresh and the partner emails.";
        let t = SessionTranscript {
            channels: vec![
                ch("mic", vec![seg(body, 0.0, 200.0)]),
                ch("system", vec![seg("Sounds good to me.", 200.0, 400.0)]),
            ],
        };
        let d = decide(signals_from(&t), RouterPolicy::default());
        assert!(d.run_summarise);
        assert!(d.run_extract_tasks);
        assert!(d.run_extract_memories);
        assert!(d.run_find_decisions);
        assert!(d.run_autoname);
    }

    #[test]
    fn too_short_skips_summarise_and_memories() {
        let t = SessionTranscript {
            channels: vec![ch("mic", vec![seg("Hello world.", 0.0, 5.0)])],
        };
        let d = decide(signals_from(&t), RouterPolicy::default());
        assert!(!d.run_summarise);
        assert!(!d.run_extract_memories);
    }

    #[test]
    fn empty_transcript_is_a_noop_aside_from_autoname() {
        let t = SessionTranscript { channels: vec![] };
        let d = decide(signals_from(&t), RouterPolicy::default());
        assert!(!d.run_summarise);
        assert!(!d.run_extract_tasks);
        assert!(!d.run_extract_memories);
        assert!(!d.run_find_decisions);
        assert!(d.run_autoname);
    }

    #[test]
    fn participants_counted_only_when_channel_has_words() {
        let t = SessionTranscript {
            channels: vec![
                ch("mic", vec![seg("alice speaks", 0.0, 200.0)]),
                ch("system", vec![seg("", 100.0, 200.0)]),
            ],
        };
        assert_eq!(signals_from(&t).participants, 1);
    }
}
