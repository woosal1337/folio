//! Speech-to-text transcription backends.
//!
//! [`Transcriber`] is the trait the rest of the app talks to. Concrete
//! implementations live in submodules: [`openai`] for the hosted Whisper
//! API and [`stub`] for tests and offline use.

pub mod openai;
pub mod stub;

use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use openai::OpenAiTranscriber;
pub use stub::StubTranscriber;

use crate::error::Result;

/// A timestamped slice of recognised speech.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct TranscriptSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub text: String,
}

/// A full transcript: the ordered sequence of segments and the language
/// the backend identified.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct Transcript {
    pub language: Option<String>,
    pub segments: Vec<TranscriptSegment>,
}

/// Outcome of a transcription run: the transcript itself plus the
/// session it belongs to and the on-disk JSON it was persisted to.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct TranscriptionResult {
    pub session_dir: std::path::PathBuf,
    pub transcript_path: std::path::PathBuf,
    pub transcript: Transcript,
}

impl Transcript {
    pub fn full_text(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Persist the transcript as pretty-printed JSON.
    pub fn write_json(&self, path: &std::path::Path) -> crate::error::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            crate::error::AttuneError::Transcription(format!("could not serialize transcript: {e}"))
        })?;
        std::fs::write(path, json).map_err(|e| {
            crate::error::AttuneError::Transcription(format!(
                "could not write transcript {}: {e}",
                path.display()
            ))
        })?;
        Ok(())
    }
}

/// Transcribe an on-disk audio file. Backends are responsible for any
/// re-encoding the underlying service needs; callers pass the path to the
/// captured WAV.
pub trait Transcriber: Send + Sync {
    fn transcribe(&self, audio_path: &Path, language_hint: Option<&str>) -> Result<Transcript>;
}
