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

impl Transcript {
    pub fn full_text(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Transcribe an on-disk audio file. Backends are responsible for any
/// re-encoding the underlying service needs; callers pass the path to the
/// captured WAV.
pub trait Transcriber: Send + Sync {
    fn transcribe(&self, audio_path: &Path, language_hint: Option<&str>) -> Result<Transcript>;
}
