//! OpenAI Whisper transcription backend.
//!
//! This is the scaffold for the v0 transcription path: it owns the API
//! key and the language hint, and exposes the [`Transcriber`] trait. The
//! HTTP call is intentionally left unimplemented for now — wiring it up
//! to the actual `/v1/audio/transcriptions` endpoint will land in its
//! own PR alongside the integration tests that exercise it.

use std::path::Path;

use crate::error::{AttuneError, Result};
use crate::transcription::{Transcriber, Transcript};

const DEFAULT_MODEL: &str = "whisper-1";
const DEFAULT_ENDPOINT: &str = "https://api.openai.com/v1/audio/transcriptions";

pub struct OpenAiTranscriber {
    api_key: String,
    model: String,
    endpoint: String,
}

impl OpenAiTranscriber {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: DEFAULT_MODEL.into(),
            endpoint: DEFAULT_ENDPOINT.into(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }
}

impl Transcriber for OpenAiTranscriber {
    fn transcribe(&self, _audio_path: &Path, _language_hint: Option<&str>) -> Result<Transcript> {
        if self.api_key.is_empty() {
            return Err(AttuneError::Transcription(
                "OpenAI API key is empty — set it in Settings".into(),
            ));
        }
        // The actual multipart POST to `self.endpoint` lands in the
        // transcription-pipeline PR (week 2 of v0). For now the method
        // returns NotImplemented so callers see a clear error rather
        // than silently no-oping.
        Err(AttuneError::Transcription(
            "OpenAI Whisper backend not yet wired up; tracked for v0 week 2".into(),
        ))
    }
}
