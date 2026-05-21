//! Shared app state. Owned by `App`, mutated by screens.

// Helpers like Screen::title and format_bytes are intentionally part of the
// shared API even if not all callers use them yet.
#![allow(dead_code)]

use std::path::PathBuf;
use std::time::Instant;

use attune_core::audio::{
    list_input_devices, CaptureArtifacts, CaptureConfig, CaptureSession, DeviceInfo,
};
use serde::{Deserialize, Serialize};

use crate::notes::NotesStore;
use crate::playback::Player;
use crate::tasks::TaskStore;
use crate::transcription::{TranscriberKind, TranscriptStore};

/// State that survives across app launches.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Persisted {
    pub mic_device: Option<String>,
    pub system_audio_enabled: bool,
    pub output_dir: PathBuf,
    pub notes_dir: PathBuf,
    pub tasks_path: PathBuf,
    pub transcripts_dir: PathBuf,
    pub active_screen: Screen,
    pub transcriber: TranscriberKind,
    pub openai_api_key: String,
}

impl Default for Persisted {
    fn default() -> Self {
        Self {
            mic_device: None,
            system_audio_enabled: true,
            output_dir: default_attune_subdir("Recordings"),
            notes_dir: default_attune_subdir("Notes"),
            tasks_path: default_attune_subdir("Tasks").join("tasks.json"),
            transcripts_dir: default_attune_subdir("Transcripts"),
            active_screen: Screen::Record,
            transcriber: TranscriberKind::default(),
            openai_api_key: String::new(),
        }
    }
}

fn default_attune_subdir(name: &str) -> PathBuf {
    home_dir()
        .map(|h| h.join("Documents").join("Attune").join(name))
        .unwrap_or_else(|| PathBuf::from(format!("./{}", name)))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// The current screen.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Screen {
    Record,
    Library,
    Transcripts,
    Editor,
    Tasks,
    Settings,
}

impl Screen {
    pub fn all() -> &'static [Screen] {
        &[
            Screen::Record,
            Screen::Library,
            Screen::Transcripts,
            Screen::Editor,
            Screen::Tasks,
            Screen::Settings,
        ]
    }

    pub fn title(self) -> &'static str {
        match self {
            Screen::Record => "Record",
            Screen::Library => "Library",
            Screen::Transcripts => "Transcripts",
            Screen::Editor => "Editor",
            Screen::Tasks => "Tasks",
            Screen::Settings => "Settings",
        }
    }

    /// Whether the screen should use the full content width rather than the
    /// 720 px reading column. Used for multi-pane screens (editor with file
    /// rail, kanban with three columns) where extra width matters.
    pub fn wants_full_width(self) -> bool {
        matches!(self, Screen::Editor | Screen::Tasks | Screen::Library)
    }
}

/// In-memory state that does not persist.
pub struct Runtime {
    pub devices: Vec<DeviceInfo>,
    pub last_error: Option<String>,
    pub session: Option<CaptureSession>,
    pub recording_started: Option<Instant>,
    pub history: Vec<RecordingSummary>,
    pub player: Option<Player>,
    pub expanded_recording: Option<PathBuf>,
    pub notes: NotesStore,
    pub tasks: TaskStore,
    pub transcripts: TranscriptStore,
}

impl Runtime {
    pub fn new(persisted: &Persisted) -> Self {
        let notes = NotesStore::load(&persisted.notes_dir);
        let tasks = TaskStore::load(&persisted.tasks_path);
        let transcripts = TranscriptStore::load(&persisted.transcripts_dir);
        Self {
            devices: Vec::new(),
            last_error: None,
            session: None,
            recording_started: None,
            history: Vec::new(),
            player: Player::new(),
            expanded_recording: None,
            notes,
            tasks,
            transcripts,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RecordingSummary {
    pub session_dir: PathBuf,
    pub label: String,
    pub duration_seconds: i64,
    pub mic_bytes: Option<u64>,
    pub system_bytes: Option<u64>,
    pub mic_sample_rate: Option<u32>,
    pub system_sample_rate: Option<u32>,
}

impl Runtime {
    pub fn is_recording(&self) -> bool {
        self.session.is_some()
    }

    pub fn elapsed_label(&self) -> String {
        match self.recording_started {
            Some(t) => {
                let secs = t.elapsed().as_secs();
                let m = secs / 60;
                let s = secs % 60;
                format!("{m:02}:{s:02}")
            }
            None => "00:00".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Actions performed by the UI on shared state.
// ---------------------------------------------------------------------------

pub fn refresh_devices(rt: &mut Runtime, persisted: &mut Persisted) {
    match list_input_devices() {
        Ok(d) => {
            // If the persisted device is no longer present, drop it.
            if let Some(name) = &persisted.mic_device {
                if !d.iter().any(|x| &x.name == name) {
                    persisted.mic_device = None;
                }
            }
            rt.devices = d;
        }
        Err(e) => {
            rt.last_error = Some(format!("device enumeration: {e}"));
            rt.devices = Vec::new();
        }
    }
}

pub fn refresh_history(rt: &mut Runtime, output_dir: &std::path::Path) {
    rt.history = scan_history(output_dir);
}

pub fn start_recording(rt: &mut Runtime, persisted: &Persisted) {
    rt.last_error = None;
    let config = CaptureConfig {
        mic_enabled: true,
        system_enabled: persisted.system_audio_enabled,
        mic_device_name: persisted.mic_device.clone(),
        target_sample_rate: None,
        output_dir: persisted.output_dir.clone(),
    };

    match CaptureSession::start(config) {
        Ok(session) => {
            let channels = session.channels_active();
            if channels.is_empty() {
                rt.last_error = Some(
                    "No capture channels available. Check microphone permission in System Settings → Privacy."
                        .into(),
                );
                return;
            }
            rt.recording_started = Some(Instant::now());
            rt.session = Some(session);
        }
        Err(e) => {
            rt.last_error = Some(format!("Could not start recording: {e}"));
        }
    }
}

pub fn stop_recording(rt: &mut Runtime) {
    let Some(session) = rt.session.take() else {
        return;
    };
    match session.stop() {
        Ok(artifacts) => {
            rt.history.insert(0, summarize(&artifacts));
            rt.history.truncate(50);
        }
        Err(e) => {
            rt.last_error = Some(format!("Stop failed: {e}"));
        }
    }
    rt.recording_started = None;
}

fn summarize(art: &CaptureArtifacts) -> RecordingSummary {
    let label = art
        .session_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "session".into());
    let duration_seconds = (art.stopped_at - art.started_at).num_seconds();
    let (mic_bytes, mic_sample_rate) = match &art.mic_path {
        Some(p) => (
            std::fs::metadata(p).ok().map(|m| m.len()),
            wav_sample_rate(p),
        ),
        None => (None, None),
    };
    let (system_bytes, system_sample_rate) = match &art.system_path {
        Some(p) => (
            std::fs::metadata(p).ok().map(|m| m.len()),
            wav_sample_rate(p),
        ),
        None => (None, None),
    };
    RecordingSummary {
        session_dir: art.session_dir.clone(),
        label,
        duration_seconds,
        mic_bytes,
        system_bytes,
        mic_sample_rate,
        system_sample_rate,
    }
}

fn scan_history(output_dir: &std::path::Path) -> Vec<RecordingSummary> {
    let Ok(entries) = std::fs::read_dir(output_dir) else {
        return Vec::new();
    };
    let mut sessions: Vec<RecordingSummary> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let label = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "session".into());
        let mic_path = path.join("mic.wav");
        let sys_path = path.join("system.wav");
        let mic_bytes = std::fs::metadata(&mic_path).ok().map(|m| m.len());
        let system_bytes = std::fs::metadata(&sys_path).ok().map(|m| m.len());
        if mic_bytes.is_none() && system_bytes.is_none() {
            continue;
        }
        let mic_sample_rate = wav_sample_rate(&mic_path);
        let system_sample_rate = wav_sample_rate(&sys_path);
        let duration_seconds = wav_duration_seconds(&mic_path)
            .or_else(|| wav_duration_seconds(&sys_path))
            .unwrap_or(0);
        sessions.push(RecordingSummary {
            session_dir: path,
            label,
            duration_seconds,
            mic_bytes,
            system_bytes,
            mic_sample_rate,
            system_sample_rate,
        });
    }
    sessions.sort_by(|a, b| b.label.cmp(&a.label));
    sessions.truncate(200);
    sessions
}

fn wav_sample_rate(path: &std::path::Path) -> Option<u32> {
    Some(hound::WavReader::open(path).ok()?.spec().sample_rate)
}

fn wav_duration_seconds(path: &std::path::Path) -> Option<i64> {
    let reader = hound::WavReader::open(path).ok()?;
    let spec = reader.spec();
    let frames = reader.duration() as u64;
    if spec.sample_rate == 0 {
        return None;
    }
    Some((frames / spec.sample_rate as u64) as i64)
}

// ---------------------------------------------------------------------------
// Formatters
// ---------------------------------------------------------------------------

pub fn format_duration(secs: i64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        let m = secs / 60;
        let s = secs % 60;
        format!("{m}m {s:02}s")
    } else {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        format!("{h}h {m:02}m")
    }
}

pub fn format_bytes(b: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if b >= GB {
        format!("{:.1} GB", b as f64 / GB as f64)
    } else if b >= MB {
        format!("{:.1} MB", b as f64 / MB as f64)
    } else if b >= KB {
        format!("{:.1} kB", b as f64 / KB as f64)
    } else {
        format!("{b} B")
    }
}

pub fn format_khz(hz: u32) -> String {
    let khz = hz as f64 / 1000.0;
    if (khz.fract()).abs() < 0.05 {
        format!("{:.0} kHz", khz)
    } else {
        format!("{:.1} kHz", khz)
    }
}

#[cfg(target_os = "macos")]
pub fn reveal_in_finder(path: &std::path::Path) -> std::io::Result<()> {
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
}

#[cfg(not(target_os = "macos"))]
pub fn reveal_in_finder(_path: &std::path::Path) -> std::io::Result<()> {
    Ok(())
}
