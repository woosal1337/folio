//! Apply diarization speaker labels to a session transcript (GET-189 P1).
//!
//! Runs the diarizer over a session's `system.wav` and tags each
//! system-channel transcript segment with its speaker index (by max time
//! overlap). The mic channel is left untouched — it is the user ("You")
//! by definition. This is what turns the v0 "You / Others" split into
//! per-speaker labelling in the UI.
//!
//! Called from the transcription command (on every transcribe /
//! re-transcribe) and from `attune-cli diarize-transcript` (to apply it
//! to an existing recording without re-running Whisper).

use std::collections::BTreeSet;
use std::path::Path;

use crate::diarization::models::DiarizationModelStore;
use crate::diarization::runtime::{
    assign_speakers_by_overlap, DiarizationError, DiarizationOptions, DiarizationRuntime,
};
use crate::transcription::SessionTranscript;

/// What a labelling pass did.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiarizationOutcome {
    /// Distinct speakers found on the system channel.
    pub num_speakers: usize,
    /// System-channel segments that received a speaker label.
    pub num_labeled: usize,
    /// Total system-channel segments considered.
    pub num_segments: usize,
}

/// Diarize `<session_dir>/system.wav` and tag the system-channel
/// segments of `transcript` in place. Uses the default model store; if
/// the models aren't downloaded, returns
/// [`DiarizationError::ModelsNotDownloaded`] so the caller can skip
/// gracefully and leave the transcript unlabelled.
///
/// Speaker indices are the diarizer's raw cluster ids (stable within one
/// recording); the UI relabels them "Speaker 1/2/3…" by first appearance.
pub fn label_system_channel(
    session_dir: &Path,
    transcript: &mut SessionTranscript,
    opts: &DiarizationOptions,
) -> Result<DiarizationOutcome, DiarizationError> {
    let store = DiarizationModelStore::default_location();
    let runtime = DiarizationRuntime::from_store(&store, opts)?;

    let system_wav = session_dir.join("system.wav");
    if !system_wav.is_file() {
        // No system audio (mic-only recording) — nothing to diarize.
        return Ok(DiarizationOutcome::default());
    }

    let diarized = runtime.diarize_wav(&system_wav)?;

    let mut speakers: BTreeSet<i32> = BTreeSet::new();
    let mut outcome = DiarizationOutcome::default();
    for channel in transcript
        .channels
        .iter_mut()
        .filter(|c| c.channel == "system")
    {
        outcome.num_segments += channel.segments.len();
        let spans: Vec<(f64, f64)> = channel
            .segments
            .iter()
            .map(|s| (s.start_seconds, s.end_seconds))
            .collect();
        let assigned = assign_speakers_by_overlap(&spans, &diarized);
        for (seg, spk) in channel.segments.iter_mut().zip(assigned) {
            seg.speaker = spk;
            if let Some(s) = spk {
                speakers.insert(s);
                outcome.num_labeled += 1;
            }
        }
    }
    outcome.num_speakers = speakers.len();
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diarization::runtime::DiarizedSegment;
    use crate::transcription::{ChannelTranscript, TranscriptSegment};

    fn seg(start: f64, end: f64) -> TranscriptSegment {
        TranscriptSegment {
            start_seconds: start,
            end_seconds: end,
            text: "x".into(),
            speaker: None,
        }
    }

    #[test]
    fn overlap_assignment_labels_each_span() {
        // Two diarized speakers; three transcript spans.
        let diar = vec![
            DiarizedSegment {
                start_secs: 0.0,
                end_secs: 5.0,
                speaker: 0,
            },
            DiarizedSegment {
                start_secs: 5.0,
                end_secs: 10.0,
                speaker: 1,
            },
        ];
        let spans = [(0.5, 2.0), (6.0, 8.0), (4.6, 4.9)];
        let got = assign_speakers_by_overlap(&spans, &diar);
        assert_eq!(got, vec![Some(0), Some(1), Some(0)]);
    }

    #[test]
    fn empty_diarization_leaves_none() {
        let got = assign_speakers_by_overlap(&[(0.0, 1.0)], &[]);
        assert_eq!(got, vec![None]);
    }

    #[test]
    fn labels_only_system_channel_segments() {
        let mut t = SessionTranscript {
            channels: vec![
                ChannelTranscript {
                    channel: "mic".into(),
                    language: None,
                    segments: vec![seg(0.0, 1.0)],
                },
                ChannelTranscript {
                    channel: "system".into(),
                    language: None,
                    segments: vec![seg(0.5, 2.0), seg(6.0, 8.0)],
                },
            ],
        };
        let diar = vec![
            DiarizedSegment {
                start_secs: 0.0,
                end_secs: 5.0,
                speaker: 3,
            },
            DiarizedSegment {
                start_secs: 5.0,
                end_secs: 10.0,
                speaker: 7,
            },
        ];
        // Apply the alignment directly (no model IO).
        for ch in t.channels.iter_mut().filter(|c| c.channel == "system") {
            let spans: Vec<(f64, f64)> = ch
                .segments
                .iter()
                .map(|s| (s.start_seconds, s.end_seconds))
                .collect();
            for (s, spk) in ch
                .segments
                .iter_mut()
                .zip(assign_speakers_by_overlap(&spans, &diar))
            {
                s.speaker = spk;
            }
        }
        // mic untouched, system labelled.
        assert_eq!(t.channels[0].segments[0].speaker, None);
        assert_eq!(t.channels[1].segments[0].speaker, Some(3));
        assert_eq!(t.channels[1].segments[1].speaker, Some(7));
    }
}
