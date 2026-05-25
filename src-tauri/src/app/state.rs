//! Tauri-managed application state. Held across command invocations via
//! `app.state::<AppState>()`. Anything that must survive between IPC
//! calls lives here. UI-only state lives in the React frontend.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use attune_core::audio::{CaptureSession, RecordingStatus};
use attune_core::memory::MemoryStore;
use attune_core::storage::{Settings, SettingsStore};
use parking_lot::Mutex;
use tracing::warn;

/// Process-wide state.
///
/// Each piece is wrapped in its own `parking_lot::Mutex` so commands lock
/// only what they touch. The [`SettingsStore`] owns the on-disk path and
/// performs atomic writes; it is itself stateless and so does not need a
/// mutex.
pub struct AppState {
    pub settings: Mutex<Settings>,
    pub settings_store: SettingsStore,
    pub session: Mutex<Option<CaptureSession>>,
    pub recording_started: Mutex<Option<Instant>>,
    /// Cached MemoryStore handle keyed by the directory it was opened
    /// against. Each open costs ~5-15ms (SQLite connection + FTS5
    /// init + sqlite-vec auto-extension); a /memory page mount fires
    /// half a dozen IPC calls, so caching saves 60-180ms of latency
    /// per page load. Invalidates + reopens when `settings.memory_dir`
    /// changes. v2 roadmap finding R14.
    memory_store: Mutex<Option<(PathBuf, Arc<MemoryStore>)>>,
}

impl AppState {
    /// Construct application state from a settings store. Loads the
    /// current settings from disk (defaults if the file is missing or
    /// malformed). Use [`AppState::new_default`] for the production
    /// location; tests use [`AppState::new`] with a custom store.
    pub fn new(settings_store: SettingsStore) -> Self {
        let settings = settings_store.load();
        Self {
            settings: Mutex::new(settings),
            settings_store,
            session: Mutex::new(None),
            recording_started: Mutex::new(None),
            memory_store: Mutex::new(None),
        }
    }

    /// Resolve the shared [`MemoryStore`] for the configured memory
    /// directory. Lazy on first call; subsequent calls reuse the
    /// cached handle unless the directory changed (rare — only when
    /// the user edits the path in Settings).
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

    /// Construct state using the platform's default settings location.
    pub fn new_default() -> Self {
        Self::new(SettingsStore::default_location())
    }

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
