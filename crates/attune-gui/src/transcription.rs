//! Transcription scaffolding. A trait-based abstraction so we can swap
//! providers without touching the UI. Today this is a scaffold — concrete
//! providers land in the next milestone.
//!
//! - `LocalWhisper` (planned): whisper.cpp via `whisper-rs`, Metal-accelerated.
//!   Distil-large-v3 model downloaded once on first transcription. Default
//!   for the privacy-first product positioning.
//! - `OpenAi`  (planned): POST audio to OpenAI's `audio/transcriptions`
//!   endpoint. User supplies their own API key in settings. Faster to wire
//!   up; useful as a bridge while local Whisper compiles in.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

/// Which provider the user has chosen for transcription.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranscriberKind {
    #[default]
    LocalWhisper,
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

/// One segment of a transcript. Mirrors what Whisper returns: start/end ms
/// + text + optional speaker label.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    #[serde(default)]
    pub speaker: Option<String>,
}

/// A completed transcript. Persisted as JSON next to the recording.
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

/// Status of an in-flight transcription job, surfaced in the UI.
#[derive(Clone, Debug)]
pub enum TranscriptionStatus {
    Idle,
    Queued,
    Downloading { progress: f32 },
    Running { progress: f32 },
    Error(String),
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
        // Replace or insert.
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
