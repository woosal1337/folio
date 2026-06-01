//! Tauri-managed application state. Held across command invocations via
//! `app.state::<AppState>()`. Anything that must survive between IPC
//! calls lives here. UI-only state lives in the React frontend.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Utc};

use attune_core::audio::{CaptureSession, RecordingStatus};
use attune_core::memory::MemoryStore;
use attune_core::storage::{Settings, SettingsStore};
use parking_lot::Mutex;
use tracing::warn;

use crate::app::meeting_watcher::DetectedMeeting;

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
    /// The most recently auto-detected meeting awaiting a user decision
    /// in the HUD (GET-143). The watcher writes it before opening the
    /// HUD window; the HUD reads it on mount via `get_pending_meeting`,
    /// and Take Notes / Dismiss / Don't-ask clear it.
    pub pending_meeting: Mutex<Option<DetectedMeeting>>,
    /// Multi-part note accumulator for pause/resume (GET-149). `None`
    /// for a normal single-shot recording — the single-shot path never
    /// touches this. Becomes `Some` the first time the user pauses, and
    /// is cleared on the final stop after the parts are merged.
    pub active_note: Mutex<Option<PausedNote>>,
    /// Stop signal for the live-transcript preview loop (GET-160). Set
    /// when a capture starts (when local Whisper is configured) and
    /// flipped to `true` on stop/pause so the background thread exits.
    pub live_transcript_stop: Mutex<Option<Arc<std::sync::atomic::AtomicBool>>>,
    /// JoinHandle for the live-transcript thread (GET-178). Stored so
    /// `RunEvent::ExitRequested` can join it after flipping the stop
    /// signal, rather than letting a detached thread run past app exit.
    pub live_transcript_thread: Mutex<Option<std::thread::JoinHandle<()>>>,
    /// Active mic-monitor (Settings test-mic loopback). None when idle.
    pub mic_monitor: Mutex<Option<attune_core::audio::mic_monitor::MicMonitor>>,
}

/// A recording that spans multiple capture segments because the user
/// paused and resumed. Each segment is finalized into its own WAV; the
/// final stop merges them into one continuous `mic.wav` / `system.wav`
/// in [`PausedNote::dir`]. GET-149.
#[derive(Debug, Clone)]
pub struct PausedNote {
    /// The note's directory — the first segment's session dir. Live
    /// notes and the merged WAVs live here; later segments capture into
    /// `dir/parts/NNN/`.
    pub dir: PathBuf,
    /// Finalized mic WAVs, in capture order (part 0 is `dir/mic.wav`).
    pub mic_parts: Vec<PathBuf>,
    /// Finalized system WAVs, in capture order.
    pub system_parts: Vec<PathBuf>,
    /// Total elapsed seconds of the finalized parts, so the resumed
    /// session reports a continuous elapsed time.
    pub base_offset_secs: u64,
    /// Index of the next part subdirectory (`dir/parts/NNN/`).
    pub next_part: usize,
    /// Wall-clock start of the first segment, for the final artifacts.
    pub started_at: DateTime<Utc>,
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
            pending_meeting: Mutex::new(None),
            active_note: Mutex::new(None),
            live_transcript_stop: Mutex::new(None),
            live_transcript_thread: Mutex::new(None),
            mic_monitor: Mutex::new(None),
        }
    }

    /// Signal the live-transcript preview loop (if any) to stop. Called
    /// from stop/pause. Idempotent.
    pub fn stop_live_transcript(&self) {
        if let Some(flag) = self.live_transcript_stop.lock().take() {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Join the live-transcript thread after signalling it to stop
    /// (GET-178). Logs a warning on panic; safe to call if the thread
    /// was never started or has already exited.
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
    ///
    /// # Lock-acquisition order (GET-179)
    ///
    /// This function holds three guards simultaneously. Canonical order
    /// (must be followed at ALL multi-lock call sites to prevent deadlock):
    ///   1. `session`
    ///   2. `recording_started`
    ///   3. `active_note`
    ///
    /// All guards drop at the end of this sync fn — no `.await` inside.
    pub fn recording_status(&self) -> RecordingStatus {
        // 1. session
        let session = self.session.lock();
        // 2. recording_started
        let started = self.recording_started.lock();
        // 3. active_note
        let note = self.active_note.lock();
        let recording = session.is_some();
        // Elapsed is continuous across pause/resume: the finalized parts'
        // duration plus the current segment's running time.
        let base = note.as_ref().map(|n| n.base_offset_secs).unwrap_or(0);
        let current = started.map(|t| t.elapsed().as_secs()).unwrap_or(0);
        let elapsed_secs = base + current;
        // Paused = a note is open but no segment is capturing.
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
        // Report the NOTE directory (stable across segments) so the live
        // notes editor keeps autosaving to one place; fall back to the
        // session dir for a normal single-shot recording.
        let session_dir = note
            .as_ref()
            .map(|n| n.dir.clone())
            .or_else(|| session.as_ref().map(|s| s.session_dir().clone()))
            .map(|p| p.to_string_lossy().into_owned());
        // GET-171: check if VPIO started but is delivering silence.
        let vpio_silent = session
            .as_ref()
            .map(|s| s.is_vpio_silent())
            .unwrap_or(false);
        // GET-211: flag when the current segment exceeds the auto-segment
        // threshold so the recording store can trigger a roll-over.
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
            vpio_silent,
            needs_segment,
        }
    }
}
