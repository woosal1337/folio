//! Agent router — decide which extraction agents to fire from the
//! transcript's shape. v2 finding 029 / GET-55.
//!
//! Today the post-transcription pipeline fans out to every enabled
//! agent (summarize, extract-tasks, extract-memories, autoname). For a
//! 90-second voice memo, half of those are noise — there are no tasks,
//! no decisions, no participants worth memorising. This router takes
//! a small set of cheap signals from the transcript and decides which
//! agents are worth running. Saves money on the BYO-key audience and
//! cuts UI clutter for everyone.
//!
//! All signals come from transcript metadata that's already in
//! memory after `read_transcript`:
//!
//!   * Duration in seconds.
//!   * Word count (joined transcript text length, whitespace-split).
//!   * Participant count = number of channels with at least one
//!     non-empty segment (today: mic + system, so 1 or 2).
//!   * Mic-vs-system token ratio (rough "monologue vs conversation").
//!
//! The rules deliberately err on the side of "run the agent" — we'd
//! rather waste a few cents than silently drop a real task. A 0-arg
//! `RouterPolicy::default()` returns the production thresholds; tests
//! tweak them.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::super::transcription::SessionTranscript;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct RouterPolicy {
    /// Below this many seconds the recording is a voice memo. Skip
    /// task / decision extraction; keep summarise + autoname.
    pub voice_memo_max_secs: f64,
    /// Below this many words we don't bother summarising — the user
    /// can read the transcript in under 10s.
    pub summarise_min_words: usize,
    /// Below this many words the memory pass is unlikely to find a
    /// claim worth keeping (small-talk threshold).
    pub memories_min_words: usize,
    /// Recordings with only one participating channel are monologues;
    /// the decision-finder is built for back-and-forth. Skip on
    /// monologues.
    pub decisions_min_participants: usize,
}

impl Default for RouterPolicy {
    fn default() -> Self {
        Self {
            voice_memo_max_secs: 120.0,
            summarise_min_words: 30,
            memories_min_words: 50,
            decisions_min_participants: 2,
        }
    }
}

/// Signals extracted from the transcript that the rules act on.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct TranscriptSignals {
    pub duration_secs: f64,
    pub word_count: usize,
    pub participants: usize,
}

/// Which extraction agents the router recommends running. The caller
/// AND-combines this with the user's own auto-run toggles.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct RouterDecision {
    pub run_summarise: bool,
    pub run_extract_tasks: bool,
    pub run_extract_memories: bool,
    pub run_find_decisions: bool,
    pub run_autoname: bool,
}

/// Compute the transcript signals from a SessionTranscript.
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

/// Decide which agents to fire from the signals + policy. Pure
/// function, no IO.
pub fn decide(signals: TranscriptSignals, policy: RouterPolicy) -> RouterDecision {
    let is_voice_memo = signals.duration_secs <= policy.voice_memo_max_secs;
    let too_short_to_summarise = signals.word_count < policy.summarise_min_words;
    let too_short_for_memories = signals.word_count < policy.memories_min_words;
    let solo_speaker = signals.participants < policy.decisions_min_participants;

    RouterDecision {
        run_summarise: !too_short_to_summarise,
        // Voice memos and monologues rarely carry committed action
        // items; the user is talking to themselves. Skip.
        run_extract_tasks: !is_voice_memo,
        run_extract_memories: !too_short_for_memories,
        // Decisions require at least two participants by definition.
        run_find_decisions: !solo_speaker && !is_voice_memo,
        // Autoname is dirt cheap and the library row needs a label;
        // always run.
        run_autoname: true,
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
