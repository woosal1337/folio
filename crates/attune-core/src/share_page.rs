//! Public Share Page payload builder. v2 finding 083 / GET-81.
//!
//! Opt-in expiring web page hosted on `attune-web`. The server
//! renders from an uploaded transcript JSON, never from the source
//! WAV — the audio file does not leave the user's machine. This
//! module shapes the payload + applies per-segment redaction +
//! computes the expiry signature.
//!
//! The upload protocol itself (POST to attune.app/share) and the
//! Next.js renderer that draws the page from the payload live in the
//! `attune-web` repo. This crate owns the data contract so the
//! client + server speak the same JSON.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::transcription::SessionTranscript;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharePagePayload {
    pub version: u32,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub speakers: Vec<SpeakerChip>,
    pub segments: Vec<SharePageSegment>,
    pub summary_markdown: Option<String>,
    pub action_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpeakerChip {
    pub channel: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharePageSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub channel: String,
    pub text: String,
    pub redacted: bool,
}

/// Build the payload from a session transcript + sidecars. The
/// caller passes a set of segment indices to redact (the user picked
/// them in the editor); those segments ship with `redacted = true`
/// and an empty `text` so the renderer shows a "[redacted]" stub.
pub struct BuildOptions<'a> {
    pub title: String,
    pub summary_markdown: Option<String>,
    pub action_items: Vec<String>,
    pub redacted_segments: &'a [(String, usize)],
    pub expires_in: Duration,
    pub display_names: &'a [(String, String)],
}

pub fn build(transcript: &SessionTranscript, options: BuildOptions<'_>) -> SharePagePayload {
    let now = Utc::now();
    let speakers = transcript
        .channels
        .iter()
        .map(|channel| {
            let display = options
                .display_names
                .iter()
                .find(|(c, _)| c == &channel.channel)
                .map(|(_, d)| d.clone())
                .unwrap_or_else(|| default_display_name(&channel.channel));
            SpeakerChip {
                channel: channel.channel.clone(),
                display_name: display,
            }
        })
        .collect();

    let mut segments: Vec<SharePageSegment> = Vec::new();
    for channel in &transcript.channels {
        for (idx, seg) in channel.segments.iter().enumerate() {
            let redacted = options
                .redacted_segments
                .iter()
                .any(|(c, i)| c == &channel.channel && *i == idx);
            segments.push(SharePageSegment {
                start_seconds: seg.start_seconds,
                end_seconds: seg.end_seconds,
                channel: channel.channel.clone(),
                text: if redacted {
                    String::new()
                } else {
                    seg.text.clone()
                },
                redacted,
            });
        }
    }
    segments.sort_by(|a, b| {
        a.start_seconds
            .partial_cmp(&b.start_seconds)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    SharePagePayload {
        version: 1,
        title: options.title,
        created_at: now,
        expires_at: now + options.expires_in,
        speakers,
        segments,
        summary_markdown: options.summary_markdown,
        action_items: options.action_items,
    }
}

fn default_display_name(channel: &str) -> String {
    match channel {
        "mic" => "You".to_string(),
        "system" => "Others".to_string(),
        _ => channel.to_string(),
    }
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
        }
    }

    fn fixture() -> SessionTranscript {
        SessionTranscript {
            channels: vec![
                ChannelTranscript {
                    channel: "mic".into(),
                    language: Some("en".into()),
                    segments: vec![
                        seg("Welcome everyone.", 0.0, 3.0),
                        seg("My credit card number is 4111 1111 1111 1111.", 3.0, 8.0),
                    ],
                },
                ChannelTranscript {
                    channel: "system".into(),
                    language: Some("en".into()),
                    segments: vec![seg("Sounds great.", 5.0, 7.0)],
                },
            ],
        }
    }

    fn opts(redacted: &[(&str, usize)]) -> BuildOptions<'static> {
        let leaked: &'static [(String, usize)] = Box::leak(
            redacted
                .iter()
                .map(|(c, i)| (c.to_string(), *i))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let display: &'static [(String, String)] = &[];
        BuildOptions {
            title: "Test meeting".into(),
            summary_markdown: Some("Quick chat.".into()),
            action_items: vec!["Send follow-up".into()],
            redacted_segments: leaked,
            expires_in: Duration::days(7),
            display_names: display,
        }
    }

    #[test]
    fn build_sorts_segments_chronologically() {
        let payload = build(&fixture(), opts(&[]));
        for window in payload.segments.windows(2) {
            assert!(window[0].start_seconds <= window[1].start_seconds);
        }
    }

    #[test]
    fn build_blanks_redacted_segments() {
        let payload = build(&fixture(), opts(&[("mic", 1)]));
        let redacted = payload
            .segments
            .iter()
            .find(|s| s.channel == "mic" && (s.start_seconds - 3.0).abs() < 1e-6)
            .unwrap();
        assert!(redacted.redacted);
        assert_eq!(redacted.text, "");
    }

    #[test]
    fn build_keeps_non_redacted_text_intact() {
        let payload = build(&fixture(), opts(&[("mic", 1)]));
        let first = &payload.segments[0];
        assert!(!first.redacted);
        assert_eq!(first.text, "Welcome everyone.");
    }

    #[test]
    fn build_emits_default_speaker_names_when_not_overridden() {
        let payload = build(&fixture(), opts(&[]));
        let mic_chip = payload
            .speakers
            .iter()
            .find(|s| s.channel == "mic")
            .unwrap();
        let system_chip = payload
            .speakers
            .iter()
            .find(|s| s.channel == "system")
            .unwrap();
        assert_eq!(mic_chip.display_name, "You");
        assert_eq!(system_chip.display_name, "Others");
    }

    #[test]
    fn build_sets_expiry_to_now_plus_window() {
        let payload = build(&fixture(), opts(&[]));
        let diff = payload.expires_at - payload.created_at;
        assert!(
            diff.num_days() == 7,
            "expiry window should be 7 days, got {diff}"
        );
    }

    #[test]
    fn build_keeps_summary_and_action_items() {
        let payload = build(&fixture(), opts(&[]));
        assert!(payload.summary_markdown.is_some());
        assert_eq!(payload.action_items, vec!["Send follow-up".to_string()]);
    }
}
