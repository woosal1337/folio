//! No-op transcription backend for tests and "transcription disabled" mode.
//!
//! Returns an empty [`Transcript`] regardless of input. Use this in
//! integration tests so the surrounding pipeline can be exercised
//! without depending on a network service.

use std::path::Path;

use crate::error::Result;
use crate::transcription::{Transcriber, Transcript};

#[derive(Default)]
pub struct StubTranscriber;

impl Transcriber for StubTranscriber {
    fn transcribe(&self, _audio_path: &Path, language_hint: Option<&str>) -> Result<Transcript> {
        Ok(Transcript {
            language: language_hint.map(|s| s.to_string()),
            segments: Vec::new(),
        })
    }
}
