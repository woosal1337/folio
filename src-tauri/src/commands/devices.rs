//! Audio device enumeration + pre-flight mic level check (GET-212).

use attune_core::audio::devices::{
    list_input_devices as core_list_input_devices, sample_mic_level, DeviceInfo, MicLevelResult,
};
use tracing::debug;

/// Enumerate input audio devices visible to the system.
#[tauri::command]
pub async fn list_input_devices() -> Result<Vec<DeviceInfo>, String> {
    debug!("list_input_devices");
    tauri::async_runtime::spawn_blocking(core_list_input_devices)
        .await
        .map_err(|e| format!("list_input_devices task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

/// Sample the default (or named) mic input for ~500 ms and return the
/// RMS + peak level in dBFS with a qualitative status (GET-212).
/// Runs on a blocking task because cpal stream setup touches the OS audio stack.
#[tauri::command]
pub async fn check_mic_level(device_name: Option<String>) -> Result<MicLevelResult, String> {
    debug!("check_mic_level");
    tauri::async_runtime::spawn_blocking(move || sample_mic_level(device_name.as_deref(), 500))
        .await
        .map_err(|e| format!("check_mic_level task panicked: {e}"))?
        .map_err(|e| e.to_string())
}
