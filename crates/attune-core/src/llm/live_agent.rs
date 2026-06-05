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

    pub fn should_tick(&self, now: Instant) -> bool {
        match self.last_tick {
            None => true,
            Some(last) => now.duration_since(last) >= self.tick_interval,
        }
    }

    pub fn mark_ticked(&mut self, now: Instant) {
        self.last_tick = Some(now);
    }

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
                Nudge {
                    kind: NudgeKind::Ask,
                    text: "a".into(),
                },
                Nudge {
                    kind: NudgeKind::Verify,
                    text: "b".into(),
                },
                Nudge {
                    kind: NudgeKind::Action,
                    text: "c".into(),
                },
            ],
        );
        assert_eq!(out.len(), 3);
    }
}
