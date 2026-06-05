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
