//! Transcription. A trait-based abstraction with two backends:
//!
//! - `LocalWhisper` (planned for a future session): whisper.cpp via
//!   `whisper-rs`, Metal-accelerated. Downloads a model on first use.
//! - `OpenAi` (this session): POSTs the audio to OpenAI's
//!   `audio/transcriptions` endpoint as multipart with `response_format =
//!   verbose_json` so we get per-segment timestamps. Single-file upload
//!   capped at OpenAI's 25 MB limit — long meetings will need chunking
//!   (next iteration).
//!
//! Jobs run on background threads. A `mpsc::Receiver<TranscriptionEvent>`
//! streams progress events back to the UI thread.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Which provider the user has chosen for transcription.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranscriberKind {
    LocalWhisper,
    #[default]
    OpenAi,
}

impl TranscriberKind {
    pub fn label(self) -> &'static str {
        match self {
            TranscriberKind::LocalWhisper => "Local Whisper",
            TranscriberKind::OpenAi => "OpenAI Whisper API",
        }
    }
    pub fn all() -> &'static [TranscriberKind] {
        &[TranscriberKind::LocalWhisper, TranscriberKind::OpenAi]
    }
}

/// One segment of a transcript. Mirrors what Whisper returns.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    #[serde(default)]
    pub speaker: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
}

/// A completed transcript. Persisted as JSON in the transcripts directory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transcript {
    pub id: Uuid,
    pub session_dir: PathBuf,
    pub recording_label: String,
    pub created_at: DateTime<Utc>,
    pub provider: TranscriberKind,
    pub model: String,
    pub language: Option<String>,
    pub duration_seconds: u32,
    pub segments: Vec<TranscriptSegment>,
}

impl Transcript {
    pub fn full_text(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.text.trim())
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn created_label(&self) -> String {
        let local: DateTime<Local> = self.created_at.into();
        local.format("%b %-d, %H:%M").to_string()
    }
}

/// Events streamed from the background thread.
#[derive(Clone, Debug)]
pub enum TranscriptionEvent {
    Started,
    Progress(String),
    Completed(Transcript),
    Failed(String),
}

#[derive(Default)]
pub struct TranscriptStore {
    pub dir: PathBuf,
    pub transcripts: Vec<Transcript>,
}

impl TranscriptStore {
    pub fn load(dir: &Path) -> Self {
        let mut store = Self {
            dir: dir.to_path_buf(),
            transcripts: Vec::new(),
        };
        store.reload();
        store
    }

    pub fn reload(&mut self) {
        if let Err(e) = std::fs::create_dir_all(&self.dir) {
            warn!(error = %e, "could not create transcripts dir");
            return;
        }
        let mut out: Vec<Transcript> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                match std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|s| serde_json::from_str::<Transcript>(&s).ok())
                {
                    Some(t) => out.push(t),
                    None => continue,
                }
            }
        }
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        self.transcripts = out;
    }

    pub fn for_session<'a>(&'a self, session_dir: &Path) -> Option<&'a Transcript> {
        self.transcripts
            .iter()
            .find(|t| t.session_dir == session_dir)
    }

    pub fn save(&mut self, transcript: Transcript) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let filename = format!(
            "{}.json",
            transcript
                .session_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&transcript.id.to_string())
        );
        let path = self.dir.join(filename);
        let body = serde_json::to_string_pretty(&transcript)
            .map_err(|e| std::io::Error::other(format!("serialize: {e}")))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, &path)?;
        if let Some(pos) = self
            .transcripts
            .iter()
            .position(|t| t.session_dir == transcript.session_dir)
        {
            self.transcripts[pos] = transcript;
        } else {
            self.transcripts.insert(0, transcript);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Background job orchestration
// ---------------------------------------------------------------------------

/// Input to `transcribe_session`. Both audio files for the session, plus the
/// configuration to use.
pub struct TranscriptionRequest {
    pub session_dir: PathBuf,
    pub mic_path: Option<PathBuf>,
    pub system_path: Option<PathBuf>,
    pub recording_label: String,
    pub provider: TranscriberKind,
    pub openai_api_key: String,
    /// Hint or "auto".
    pub language: String,
    /// Custom vocabulary used as a prompt to bias spelling.
    pub initial_prompt: Option<String>,
}

/// Kick off a transcription job. Returns a receiver yielding progress events.
pub fn transcribe_session(req: TranscriptionRequest) -> mpsc::Receiver<TranscriptionEvent> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(TranscriptionEvent::Started);
        match req.provider {
            TranscriberKind::OpenAi => run_openai(req, &tx),
            TranscriberKind::LocalWhisper => {
                let _ = tx.send(TranscriptionEvent::Failed(
                    "Local Whisper backend lands in a future session. Switch to OpenAI Whisper API in Settings for now."
                        .into(),
                ));
            }
        }
    });
    rx
}

fn run_openai(req: TranscriptionRequest, tx: &mpsc::Sender<TranscriptionEvent>) {
    if req.openai_api_key.trim().is_empty() {
        let _ = tx.send(TranscriptionEvent::Failed(
            "OpenAI API key missing. Add one in Settings → Transcription.".into(),
        ));
        return;
    }

    let _ = tx.send(TranscriptionEvent::Progress("preparing audio".into()));

    let mut segments_all: Vec<TranscriptSegment> = Vec::new();
    let mut detected_language: Option<String> = None;
    let mut duration_seconds: u32 = 0;

    // Mic channel first so "you" segments appear before "others" on tied
    // timestamps after the sort.
    if let Some(path) = req.mic_path.as_deref() {
        let _ = tx.send(TranscriptionEvent::Progress("transcribing mic".into()));
        match openai_transcribe_file(
            path,
            &req.openai_api_key,
            &req.language,
            req.initial_prompt.as_deref(),
        ) {
            Ok(out) => {
                detected_language.get_or_insert(out.language.clone());
                duration_seconds = duration_seconds.max(out.duration);
                for seg in out.segments {
                    segments_all.push(TranscriptSegment {
                        start_ms: seg.start_ms,
                        end_ms: seg.end_ms,
                        text: seg.text,
                        speaker: Some("you".into()),
                        language: Some(out.language.clone()),
                    });
                }
            }
            Err(e) => {
                let _ = tx.send(TranscriptionEvent::Failed(format!("mic: {e}")));
                return;
            }
        }
    }

    if let Some(path) = req.system_path.as_deref() {
        let _ = tx.send(TranscriptionEvent::Progress("transcribing system".into()));
        match openai_transcribe_file(
            path,
            &req.openai_api_key,
            &req.language,
            req.initial_prompt.as_deref(),
        ) {
            Ok(out) => {
                if detected_language.is_none() {
                    detected_language = Some(out.language.clone());
                }
                duration_seconds = duration_seconds.max(out.duration);
                for seg in out.segments {
                    segments_all.push(TranscriptSegment {
                        start_ms: seg.start_ms,
                        end_ms: seg.end_ms,
                        text: seg.text,
                        speaker: Some("others".into()),
                        language: Some(out.language.clone()),
                    });
                }
            }
            Err(e) => {
                let _ = tx.send(TranscriptionEvent::Failed(format!("system: {e}")));
                return;
            }
        }
    }

    segments_all.sort_by_key(|s| s.start_ms);

    let transcript = Transcript {
        id: Uuid::new_v4(),
        session_dir: req.session_dir.clone(),
        recording_label: req.recording_label,
        created_at: Utc::now(),
        provider: TranscriberKind::OpenAi,
        model: "whisper-1".into(),
        language: detected_language,
        duration_seconds,
        segments: segments_all,
    };

    let _ = tx.send(TranscriptionEvent::Completed(transcript));
}

// ---------------------------------------------------------------------------
// OpenAI HTTP layer
// ---------------------------------------------------------------------------

const OPENAI_LIMIT_BYTES: u64 = 25 * 1024 * 1024; // 25 MB

struct OpenAiFileResult {
    language: String,
    duration: u32,
    segments: Vec<OpenAiSegment>,
}

struct OpenAiSegment {
    start_ms: u64,
    end_ms: u64,
    text: String,
}

fn openai_transcribe_file(
    path: &Path,
    api_key: &str,
    language: &str,
    initial_prompt: Option<&str>,
) -> Result<OpenAiFileResult, String> {
    let size = std::fs::metadata(path)
        .map(|m| m.len())
        .map_err(|e| format!("stat {}: {e}", path.display()))?;
    if size > OPENAI_LIMIT_BYTES {
        return Err(format!(
            "{} is {:.1} MB, which exceeds OpenAI's 25 MB per-file limit. \
             Recording is too long for single-shot upload. \
             Chunked uploads land next session.",
            path.display(),
            size as f64 / (1024.0 * 1024.0),
        ));
    }

    info!(path = %path.display(), size, "uploading to OpenAI Whisper");

    let mut form = reqwest::blocking::multipart::Form::new()
        .file("file", path)
        .map_err(|e| format!("attach file: {e}"))?
        .text("model", "whisper-1")
        .text("response_format", "verbose_json")
        .text("temperature", "0");

    if language != "auto" && !language.is_empty() {
        form = form.text("language", language.to_string());
    }
    if let Some(prompt) = initial_prompt {
        if !prompt.trim().is_empty() {
            form = form.text("prompt", prompt.to_string());
        }
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| format!("http client: {e}"))?;

    let resp = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .map_err(|e| format!("request failed: {e}"))?;

    let status = resp.status();
    let body_text = resp.text().map_err(|e| format!("read body: {e}"))?;

    if !status.is_success() {
        error!(status = %status, body = %body_text, "OpenAI API error");
        return Err(format!("OpenAI API {}: {}", status, snippet(&body_text)));
    }

    let parsed: OpenAiResponse = serde_json::from_str(&body_text)
        .map_err(|e| format!("parse response: {e}\nbody: {}", snippet(&body_text)))?;

    let segments = parsed
        .segments
        .into_iter()
        .map(|s| OpenAiSegment {
            start_ms: (s.start * 1000.0).max(0.0) as u64,
            end_ms: (s.end * 1000.0).max(0.0) as u64,
            text: s.text,
        })
        .collect();

    Ok(OpenAiFileResult {
        language: parsed.language,
        duration: parsed.duration.max(0.0).round() as u32,
        segments,
    })
}

fn snippet(s: &str) -> String {
    if s.len() <= 200 {
        s.to_string()
    } else {
        format!("{}...", &s[..200])
    }
}

#[derive(Deserialize)]
struct OpenAiResponse {
    #[allow(dead_code)]
    text: String,
    #[serde(default)]
    language: String,
    #[serde(default)]
    duration: f64,
    #[serde(default)]
    segments: Vec<OpenAiResponseSegment>,
}

#[derive(Deserialize)]
struct OpenAiResponseSegment {
    #[serde(default)]
    start: f64,
    #[serde(default)]
    end: f64,
    #[serde(default)]
    text: String,
}
