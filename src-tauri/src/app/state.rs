use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Utc};

use folio_core::audio::{CaptureSession, RecordingStatus};
use folio_core::memory::MemoryStore;
use folio_core::storage::{Settings, SettingsStore};
use parking_lot::Mutex;
use tracing::warn;

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub settings_store: SettingsStore,
    pub session: Mutex<Option<CaptureSession>>,
    pub recording_started: Mutex<Option<Instant>>,

    memory_store: Mutex<Option<(PathBuf, Arc<MemoryStore>)>>,

    pub active_note: Mutex<Option<PausedNote>>,

    pub live_transcript_stop: Mutex<Option<Arc<std::sync::atomic::AtomicBool>>>,

    pub live_transcript_thread: Mutex<Option<std::thread::JoinHandle<()>>>,

    pub mic_monitor: Mutex<Option<folio_core::audio::mic_monitor::MicMonitor>>,
}

#[derive(Debug, Clone)]
pub struct PausedNote {
    pub dir: PathBuf,

    pub mic_parts: Vec<PathBuf>,

    pub system_parts: Vec<PathBuf>,

    pub base_offset_secs: u64,

    pub next_part: usize,

    pub started_at: DateTime<Utc>,
}

impl AppState {
    pub fn new(settings_store: SettingsStore) -> Self {
        let settings = settings_store.load();
        Self {
            settings: Mutex::new(settings),
            settings_store,
            session: Mutex::new(None),
            recording_started: Mutex::new(None),
            memory_store: Mutex::new(None),
            active_note: Mutex::new(None),
            live_transcript_stop: Mutex::new(None),
            live_transcript_thread: Mutex::new(None),
            mic_monitor: Mutex::new(None),
        }
    }

    pub fn stop_live_transcript(&self) {
        if let Some(flag) = self.live_transcript_stop.lock().take() {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub fn join_live_transcript(&self) {
        if let Some(handle) = self.live_transcript_thread.lock().take() {
            match handle.join() {
                Ok(()) => {}
                Err(_) => {
                    tracing::warn!("live-transcript thread panicked");
                }
            }
        }
    }

    pub fn memory_store(&self) -> Result<Arc<MemoryStore>, String> {
        let target = self.settings.lock().memory_dir.clone();
        let mut slot = self.memory_store.lock();
        if let Some((cached_path, store)) = slot.as_ref() {
            if cached_path == &target {
                return Ok(store.clone());
            }
            warn!(
                old = %cached_path.display(),
                new = %target.display(),
                "memory_dir changed, reopening MemoryStore",
            );
        }
        let store = MemoryStore::open(&target).map_err(|e| e.to_string())?;
        let store = Arc::new(store);
        *slot = Some((target, store.clone()));
        Ok(store)
    }

    pub fn new_default() -> Self {
        Self::new(SettingsStore::default_location())
    }

    pub fn recording_status(&self) -> RecordingStatus {
        let session = self.session.lock();

        let started = self.recording_started.lock();

        let note = self.active_note.lock();
        let recording = session.is_some();

        let base = note.as_ref().map(|n| n.base_offset_secs).unwrap_or(0);
        let current = started.map(|t| t.elapsed().as_secs()).unwrap_or(0);
        let elapsed_secs = base + current;

        let paused = session.is_none() && note.is_some();
        let channels = session
            .as_ref()
            .map(|s| {
                s.channels_active()
                    .into_iter()
                    .map(|c| c.as_str().to_string())
                    .collect()
            })
            .unwrap_or_default();

        let session_dir = note
            .as_ref()
            .map(|n| n.dir.clone())
            .or_else(|| session.as_ref().map(|s| s.session_dir().clone()))
            .map(|p| p.to_string_lossy().into_owned());

        let mic_silent = session.as_ref().map(|s| s.is_mic_silent()).unwrap_or(false);

        let needs_segment = recording && {
            let threshold = self.settings.lock().auto_segment_secs;
            match threshold {
                Some(secs) if secs > 0 => current >= secs,
                _ => false,
            }
        };
        RecordingStatus {
            recording,
            elapsed_secs,
            channels,
            session_dir,
            paused,
            mic_silent,
            needs_segment,
        }
    }
}
