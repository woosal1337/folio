//! Speech-to-text transcription backends.
//!
//! [`Transcriber`] is the trait the rest of the app talks to. Concrete
//! implementations live in submodules: [`openai`] for the hosted Whisper
//! API and [`stub`] for tests and offline use.

pub mod hallucination_filter;
pub mod local;
pub mod models;
pub mod openai;
pub mod stub;

use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use local::LocalWhisperTranscriber;
pub use models::{DownloadProgress, WhisperModel, WhisperModelStatus, WhisperModelStore};
pub use openai::OpenAiTranscriber;
pub use stub::StubTranscriber;

use crate::error::{AttuneError, Result};

/// A timestamped slice of recognised speech.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct TranscriptSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub text: String,
}

/// A full transcript for a single audio channel: the ordered sequence
/// of segments and the language the backend identified. This is the
/// shape each individual whisper run produces, before we attach a
/// channel name to it.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct Transcript {
    pub language: Option<String>,
    pub segments: Vec<TranscriptSegment>,
}

/// One channel's transcript inside a [`SessionTranscript`]. The
/// `channel` field is the same identifier used elsewhere ("mic" /
/// "system") so the UI can label it ("You" / "Others") consistently.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ChannelTranscript {
    /// "mic" or "system" — the audio channel this transcript was
    /// produced from.
    pub channel: String,
    pub language: Option<String>,
    pub segments: Vec<TranscriptSegment>,
}

/// Full per-session transcript: one [`ChannelTranscript`] per audio
/// channel that produced output. This is the shape that lives in
/// `<session_dir>/transcript.json` and the shape the frontend reads
/// and edits.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct SessionTranscript {
    pub channels: Vec<ChannelTranscript>,
}

/// Outcome of a transcription run: the per-channel transcripts plus
/// the session they belong to and the on-disk JSON they were
/// persisted to.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct TranscriptionResult {
    pub session_dir: std::path::PathBuf,
    pub transcript_path: std::path::PathBuf,
    pub session_transcript: SessionTranscript,
}

impl Transcript {
    pub fn full_text(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl SessionTranscript {
    /// Persist the transcript bundle as pretty-printed JSON.
    pub fn write_json(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            AttuneError::Transcription(format!("could not serialize transcript: {e}"))
        })?;
        std::fs::write(path, json).map_err(|e| {
            AttuneError::Transcription(format!(
                "could not write transcript {}: {e}",
                path.display()
            ))
        })?;
        Ok(())
    }

    /// Read a transcript JSON from disk.
    ///
    /// Older transcripts (single-channel, no `channels` field) are
    /// detected and lifted into the new shape with the channel labelled
    /// `"legacy"` so existing transcripts keep working through the UI
    /// without forcing the user to re-transcribe.
    pub fn read_json(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path).map_err(|e| {
            AttuneError::Transcription(format!("could not read transcript {}: {e}", path.display()))
        })?;

        if let Ok(session) = serde_json::from_str::<SessionTranscript>(&raw) {
            return Ok(session);
        }

        // Pre-multichannel transcripts were just `{ language, segments }`
        // at the top level. Lift them into the new shape with a single
        // legacy channel.
        let legacy: Transcript = serde_json::from_str(&raw).map_err(|e| {
            AttuneError::Transcription(format!(
                "could not parse transcript {}: {e}",
                path.display()
            ))
        })?;
        Ok(SessionTranscript {
            channels: vec![ChannelTranscript {
                channel: "legacy".to_string(),
                language: legacy.language,
                segments: legacy.segments,
            }],
        })
    }
}

/// Transcribe an on-disk audio file. Backends are responsible for any
/// re-encoding the underlying service needs; callers pass the path to the
/// captured WAV.
pub trait Transcriber: Send + Sync {
    fn transcribe(&self, audio_path: &Path, language_hint: Option<&str>) -> Result<Transcript>;
}
