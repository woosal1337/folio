//! Settings: read and persist user preferences.

use attune_core::storage::Settings;
use tauri::State;
use tracing::{debug, info};

use crate::app::AppState;

/// Read the current settings.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    debug!("get_settings");
    state.settings.lock().clone()
}

/// Persist new settings.
///
/// Atomic on disk: writes to a sibling temp file and renames into place,
/// so a crash mid-write cannot leave a half-written settings file.
#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    state
        .settings_store
        .save(&settings)
        .map_err(|e| e.to_string())?;
    *state.settings.lock() = settings;
    info!("settings saved");
    Ok(())
}
