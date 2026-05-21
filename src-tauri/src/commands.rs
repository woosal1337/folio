//! Tauri command handlers. Each `#[tauri::command]` function is callable
//! from the React frontend via `invoke('command_name', args)`.

use attune_core::audio::{list_input_devices as core_list_input_devices, DeviceInfo};
use tauri::State;

use crate::state::{AppState, Settings};

/// Health-check command used during scaffolding to verify the IPC bridge.
#[tauri::command]
pub fn ping(name: Option<String>) -> String {
    match name {
        Some(n) => format!("pong, {n}"),
        None => "pong".into(),
    }
}

/// Enumerate input audio devices visible to the system.
#[tauri::command]
pub fn list_input_devices() -> Result<Vec<DeviceInfo>, String> {
    core_list_input_devices().map_err(|e| e.to_string())
}

/// Read the current settings.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().clone()
}

/// Persist new settings.
#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    *state.settings.lock() = settings;
    Ok(())
}
