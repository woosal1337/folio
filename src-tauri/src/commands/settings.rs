use attune_core::cloud_guard;
use attune_core::storage::{Settings, SettingsStore};
use tauri::{Emitter, State};
use tracing::{debug, info};

use crate::app::AppState;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    debug!("get_settings");
    state.settings.lock().clone()
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    settings: Settings,
) -> Result<(), String> {
    let path = state.settings_store.path().to_path_buf();
    let settings_clone = settings.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let store = SettingsStore::new(path);
        store.save(&settings_clone)
    })
    .await
    .map_err(|e| format!("save_settings task panicked: {e}"))?
    .map_err(|e| e.to_string())?;

    cloud_guard::set_airgap(settings.privacy_mode);

    let vault_root = settings
        .output_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| settings.output_dir.clone());
    let policy = cloud_guard::load_egress_policy(&vault_root);
    cloud_guard::set_egress_policy(policy);
    let _ = app.emit("privacy-mode-changed", settings.privacy_mode);

    *state.settings.lock() = settings;
    info!("settings saved");
    Ok(())
}
