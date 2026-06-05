use std::collections::BTreeSet;
use std::path::Path;

use crate::diarization::models::DiarizationModelStore;
use crate::diarization::runtime::{
    assign_speakers_by_overlap, DiarizationError, DiarizationOptions, DiarizationRuntime,
    DiarizedSegment,
};
use crate::transcription::SessionTranscript;

#[derive(Debug, Clone, Copy, Default)]
pub struct DiarizationOutcome {
    pub num_speakers: usize,

    pub num_labeled: usize,

    pub num_segments: usize,
}

pub fn label_system_channel(
    session_dir: &Path,
    transcript: &mut SessionTranscript,
    opts: &DiarizationOptions,
) -> Result<DiarizationOutcome, DiarizationError> {
    let store = DiarizationModelStore::default_location();
    let runtime = DiarizationRuntime::from_store(&store, opts)?;

    let system_wav = session_dir.join("system.wav");
    if !system_wav.is_file() {
        return Ok(DiarizationOutcome::default());
    }

    let diarized = runtime.diarize_wav(&system_wav)?;
    Ok(assign_to_transcript(transcript, &diarized))
}

pub(crate) fn assign_to_transcript(
    transcript: &mut SessionTranscript,
    diarized: &[DiarizedSegment],
) -> DiarizationOutcome {
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
        let assigned = assign_speakers_by_overlap(&spans, diarized);
        for (seg, spk) in channel.segments.iter_mut().zip(assigned) {
            seg.speaker = spk;
            if let Some(s) = spk {
                speakers.insert(s);
                outcome.num_labeled += 1;
            }
        }
    }
    outcome.num_speakers = speakers.len();
    outcome
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
            language: None,
        }
    }

    #[test]
    fn overlap_assignment_labels_each_span() {
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

        assert_eq!(t.channels[0].segments[0].speaker, None);
        assert_eq!(t.channels[1].segments[0].speaker, Some(3));
        assert_eq!(t.channels[1].segments[1].speaker, Some(7));
    }
}
