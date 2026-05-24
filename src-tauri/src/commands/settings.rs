//! Settings: read and persist user preferences.

use attune_core::storage::{Settings, SettingsStore};
use tauri::State;
use tracing::{debug, info};

use crate::app::AppState;

/// Read the current settings. Snapshot of the in-memory cache, returns
/// instantly so this stays sync.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    debug!("get_settings");
    state.settings.lock().clone()
}

/// Persist new settings.
///
/// Atomic on disk: writes to a sibling temp file and renames into place,
/// so a crash mid-write cannot leave a half-written settings file. The
/// disk write runs on a blocking task so the Tauri command runtime is
/// not parked while the syscall is in flight.
#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    // Take a snapshot of the store path; the store itself is stateless
    // beyond that path, so we reconstruct it inside the blocking task
    // rather than holding the State reference across the await.
    let path = state.settings_store.path().to_path_buf();
    let settings_clone = settings.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let store = SettingsStore::new(path);
        store.save(&settings_clone)
    })
    .await
    .map_err(|e| format!("save_settings task panicked: {e}"))?
    .map_err(|e| e.to_string())?;

    *state.settings.lock() = settings;
    info!("settings saved");
    Ok(())
}
