//! Tauri-managed application state. Held across command invocations via
//! `app.state::<AppState>()`. Anything that must survive between IPC calls
//! lives here. UI-only state lives in the React frontend.

use std::path::PathBuf;
use std::time::Instant;

use attune_core::audio::CaptureSession;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Persisted settings. Mirrors the egui-era `Persisted` struct but lives on
/// disk as JSON and is read/written via dedicated Tauri commands.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub mic_device: Option<String>,
    pub system_audio_enabled: bool,
    pub output_dir: PathBuf,
    pub notes_dir: PathBuf,
    pub tasks_path: PathBuf,
    pub transcripts_dir: PathBuf,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_provider")]
    pub transcriber: String,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default = "default_language")]
    pub transcription_language: String,
    #[serde(default)]
    pub dictionary_terms: Vec<String>,
}

fn default_theme() -> String {
    "light".into()
}
fn default_provider() -> String {
    "openai".into()
}
fn default_language() -> String {
    "auto".into()
}

impl Default for Settings {
    fn default() -> Self {
        let attune = home_attune();
        Self {
            mic_device: None,
            system_audio_enabled: true,
            output_dir: attune.join("Recordings"),
            notes_dir: attune.join("Notes"),
            tasks_path: attune.join("Tasks").join("tasks.json"),
            transcripts_dir: attune.join("Transcripts"),
            theme: default_theme(),
            transcriber: default_provider(),
            openai_api_key: String::new(),
            transcription_language: default_language(),
            dictionary_terms: Vec::new(),
        }
    }
}

fn home_attune() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("Documents").join("Attune")
}

/// Status of an active (or absent) recording session, reported to the UI.
#[derive(Debug, Clone, Serialize)]
pub struct RecordingStatus {
    pub recording: bool,
    pub elapsed_secs: u64,
    pub channels: Vec<String>,
}

/// Process-wide state. Wraps each piece in its own Mutex so commands lock
/// only what they touch.
#[derive(Default)]
pub struct AppState {
    pub settings: Mutex<Settings>,
    pub session: Mutex<Option<CaptureSession>>,
    pub recording_started: Mutex<Option<Instant>>,
}

impl AppState {
    /// Snapshot the current recording status for the UI.
    pub fn recording_status(&self) -> RecordingStatus {
        let session = self.session.lock();
        let started = self.recording_started.lock();
        let recording = session.is_some();
        let elapsed_secs = started.map(|t| t.elapsed().as_secs()).unwrap_or(0);
        let channels = session
            .as_ref()
            .map(|s| {
                s.channels_active()
                    .into_iter()
                    .map(|c| c.as_str().to_string())
                    .collect()
            })
            .unwrap_or_default();
        RecordingStatus {
            recording,
            elapsed_secs,
            channels,
        }
    }
}
