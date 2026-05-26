//! Live in-meeting co-pilot. v2 finding 032 / GET-58.
//!
//! Opt-in side rail that runs alongside the recording. Every N
//! seconds the rolling tail of the transcript (last M segments) is
//! handed to a small LLM that emits a short list of nudges:
//!
//!   * "ask: what's the budget cap?"  — question worth surfacing now
//!   * "verify: Q4 launched in October" — fact worth confirming live
//!   * "action: Alice owns the press release" — likely action item
//!
//! Default off — the user opts in from Settings → AI. The actual
//! model call lives in the runner; this module owns the rolling tail
//! buffer + the de-dup / cooldown logic that keeps the side rail
//! quiet enough to ignore when nothing changes.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NudgeKind {
    Ask,
    Verify,
    Action,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Nudge {
    pub kind: NudgeKind,
    pub text: String,
}

pub const DEFAULT_TAIL_SECONDS: f64 = 90.0;
pub const DEFAULT_TICK_SECONDS: f64 = 20.0;
pub const DEFAULT_COOLDOWN_SECONDS: u64 = 30;

/// Rolling tail of transcript text + recent nudges. The orchestrator
/// pushes each closed segment, ticks the buffer every
/// DEFAULT_TICK_SECONDS, and asks the model when `should_tick()`
/// returns true.
pub struct LiveAgentBuffer {
    segments: VecDeque<(f64, String)>,
    tail_seconds: f64,
    last_tick: Option<Instant>,
    tick_interval: Duration,
    recent_nudges: VecDeque<(Instant, Nudge)>,
    cooldown: Duration,
}

impl LiveAgentBuffer {
    pub fn new() -> Self {
        Self {
            segments: VecDeque::new(),
            tail_seconds: DEFAULT_TAIL_SECONDS,
            last_tick: None,
            tick_interval: Duration::from_secs(DEFAULT_TICK_SECONDS as u64),
            recent_nudges: VecDeque::new(),
            cooldown: Duration::from_secs(DEFAULT_COOLDOWN_SECONDS),
        }
    }

    /// Push a closed transcript segment. The caller passes the
    /// segment's end timestamp (seconds from recording start) so the
    /// tail window can be trimmed correctly.
    pub fn push_segment(&mut self, end_seconds: f64, text: String) {
        if !text.trim().is_empty() {
            self.segments.push_back((end_seconds, text));
        }
        self.trim_tail();
    }

    fn trim_tail(&mut self) {
        if let Some((latest, _)) = self.segments.back().cloned() {
            let cutoff = latest - self.tail_seconds;
            while let Some((end, _)) = self.segments.front() {
                if *end < cutoff {
                    self.segments.pop_front();
                } else {
                    break;
                }
            }
        }
    }

    /// True when enough wall-clock time has passed since the last
    /// tick that the orchestrator should call the model again. The
    /// first tick fires immediately.
    pub fn should_tick(&self, now: Instant) -> bool {
        match self.last_tick {
            None => true,
            Some(last) => now.duration_since(last) >= self.tick_interval,
        }
    }

    /// Mark the tick as completed at `now` so `should_tick` waits
    /// the full interval before the next.
    pub fn mark_ticked(&mut self, now: Instant) {
        self.last_tick = Some(now);
    }

    /// Concatenated rolling-tail text the orchestrator hands to the
    /// model. Newlines between segments so the model sees the
    /// timing breaks.
    pub fn rolling_text(&self) -> String {
        let mut out = String::new();
        for (_, text) in &self.segments {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(text);
        }
        out
    }

    /// Filter a freshly-emitted nudge list against the recent-nudges
    /// queue. A nudge whose text matches one issued inside the
    /// cooldown window is dropped. Returns the survivors in input
    /// order.
    pub fn dedup(&mut self, now: Instant, nudges: Vec<Nudge>) -> Vec<Nudge> {
        self.prune_recent(now);
        let mut survivors = Vec::with_capacity(nudges.len());
        for nudge in nudges {
            let already = self
                .recent_nudges
                .iter()
                .any(|(_, prior)| prior.text == nudge.text);
            if !already {
                self.recent_nudges.push_back((now, nudge.clone()));
                survivors.push(nudge);
            }
        }
        survivors
    }

    fn prune_recent(&mut self, now: Instant) {
        while let Some((when, _)) = self.recent_nudges.front() {
            if now.duration_since(*when) > self.cooldown {
                self.recent_nudges.pop_front();
            } else {
                break;
            }
        }
    }
}

impl Default for LiveAgentBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_text_joins_segments_with_newlines() {
        let mut buf = LiveAgentBuffer::new();
        buf.push_segment(5.0, "hello".into());
        buf.push_segment(10.0, "world".into());
        assert_eq!(buf.rolling_text(), "hello\nworld");
    }

    #[test]
    fn empty_segments_are_ignored() {
        let mut buf = LiveAgentBuffer::new();
        buf.push_segment(5.0, "".into());
        buf.push_segment(10.0, "   ".into());
        assert_eq!(buf.rolling_text(), "");
    }

    #[test]
    fn old_segments_drop_out_of_the_tail_window() {
        let mut buf = LiveAgentBuffer::new();
        buf.push_segment(0.0, "ancient".into());
        buf.push_segment(50.0, "middle".into());
        buf.push_segment(200.0, "recent".into());
        assert_eq!(buf.rolling_text(), "recent");
    }

    #[test]
    fn should_tick_fires_on_first_call() {
        let buf = LiveAgentBuffer::new();
        assert!(buf.should_tick(Instant::now()));
    }

    #[test]
    fn should_tick_waits_for_the_interval() {
        let mut buf = LiveAgentBuffer::new();
        let now = Instant::now();
        buf.mark_ticked(now);
        assert!(!buf.should_tick(now));
    }

    #[test]
    fn dedup_drops_repeated_nudge_text_within_cooldown() {
        let mut buf = LiveAgentBuffer::new();
        let now = Instant::now();
        let first = buf.dedup(
            now,
            vec![Nudge {
                kind: NudgeKind::Ask,
                text: "what's the budget?".into(),
            }],
        );
        assert_eq!(first.len(), 1);
        let second = buf.dedup(
            now,
            vec![Nudge {
                kind: NudgeKind::Ask,
                text: "what's the budget?".into(),
            }],
        );
        assert!(second.is_empty());
    }

    #[test]
    fn dedup_lets_distinct_nudges_through() {
        let mut buf = LiveAgentBuffer::new();
        let now = Instant::now();
        let out = buf.dedup(
            now,
            vec![
                Nudge { kind: NudgeKind::Ask, text: "a".into() },
                Nudge { kind: NudgeKind::Verify, text: "b".into() },
                Nudge { kind: NudgeKind::Action, text: "c".into() },
            ],
        );
        assert_eq!(out.len(), 3);
    }
}
