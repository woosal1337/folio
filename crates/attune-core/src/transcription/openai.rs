//! OpenAI Whisper transcription backend.
//!
//! Uploads the captured WAV to `/v1/audio/transcriptions` and converts
//! the verbose-JSON response into our [`Transcript`] shape. Uses the
//! blocking reqwest client because Tauri's command handlers are
//! themselves synchronous; we live with a parked worker thread for the
//! duration of the upload.

use std::path::Path;
use std::time::Duration;

use serde::Deserialize;
use tracing::{debug, info};

use crate::error::{AttuneError, Result};
use crate::transcription::{Transcriber, Transcript, TranscriptSegment};

const DEFAULT_MODEL: &str = "whisper-1";
const DEFAULT_ENDPOINT: &str = "https://api.openai.com/v1/audio/transcriptions";
/// Whisper jobs of a meeting-length recording can take ~30 s. Give the
/// request a generous ceiling so a long meeting does not time out, but
/// still bound it so a hung connection cannot wedge the app.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(600);

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

    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    #[must_use]
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }
}

impl Transcriber for OpenAiTranscriber {
    fn transcribe(&self, audio_path: &Path, language_hint: Option<&str>) -> Result<Transcript> {
        if self.api_key.is_empty() {
            return Err(AttuneError::Transcription(
                "OpenAI API key is empty — set it in Settings".into(),
            ));
        }

        let filename = audio_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.wav")
            .to_string();

        let file_bytes = std::fs::read(audio_path).map_err(|e| {
            AttuneError::Transcription(format!(
                "could not read audio file {}: {e}",
                audio_path.display()
            ))
        })?;

        debug!(
            path = %audio_path.display(),
            bytes = file_bytes.len(),
            model = %self.model,
            "POST /v1/audio/transcriptions",
        );

        let mut form = reqwest::blocking::multipart::Form::new()
            .text("model", self.model.clone())
            // `verbose_json` returns per-segment timestamps; the default
            // `json` response only returns the concatenated text.
            .text("response_format", "verbose_json")
            .part(
                "file",
                reqwest::blocking::multipart::Part::bytes(file_bytes).file_name(filename),
            );

        // OpenAI uses ISO-639-1 codes ("en", "tr"). We pass anything that
        // is not the synthetic "auto" sentinel.
        if let Some(lang) = language_hint {
            if !lang.is_empty() && lang != "auto" {
                form = form.text("language", lang.to_string());
            }
        }

        let host = crate::cloud_guard::host_of(&self.endpoint).unwrap_or_default();
        crate::cloud_guard::ensure_allowed(host)
            .map_err(|e| AttuneError::Transcription(e.to_string()))?;

        let client = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| AttuneError::Transcription(format!("could not build HTTP client: {e}")))?;

        let response = client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .map_err(|e| AttuneError::Transcription(format!("OpenAI request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(AttuneError::Transcription(format!(
                "OpenAI returned {status}: {body}"
            )));
        }

        let parsed: WhisperResponse = response.json().map_err(|e| {
            AttuneError::Transcription(format!("could not parse OpenAI response: {e}"))
        })?;

        info!(
            language = ?parsed.language,
            segments = parsed.segments.as_ref().map(|s| s.len()).unwrap_or(0),
            "OpenAI transcription complete",
        );

        Ok(parsed.into_transcript())
    }
}

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    language: Option<String>,
    text: Option<String>,
    segments: Option<Vec<WhisperSegment>>,
}

#[derive(Debug, Deserialize)]
struct WhisperSegment {
    start: f64,
    end: f64,
    text: String,
}

impl WhisperResponse {
    fn into_transcript(self) -> Transcript {
        // The OpenAI path does not do per-segment language ID (the API
        // returns one language for the whole response), so every segment
        // inherits the response language. Local transcription tags each
        // segment with its own per-window detection (GET-190).
        let lang = self.language.clone();
        let segments = match self.segments {
            Some(segs) => segs
                .into_iter()
                .map(|s| TranscriptSegment {
                    start_seconds: s.start,
                    end_seconds: s.end,
                    text: s.text.trim().to_string(),
                    speaker: None,
                    language: lang.clone(),
                })
                .collect(),
            None => {
                // Some endpoints (or model variants) skip the segment
                // list and only return `text`. Fall back to a single
                // segment spanning the whole audio so callers still get
                // something useful.
                self.text
                    .map(|t| {
                        vec![TranscriptSegment {
                            start_seconds: 0.0,
                            end_seconds: 0.0,
                            text: t.trim().to_string(),
                            speaker: None,
                            language: lang.clone(),
                        }]
                    })
                    .unwrap_or_default()
            }
        };
        Transcript {
            language: self.language,
            segments,
        }
    }
}
