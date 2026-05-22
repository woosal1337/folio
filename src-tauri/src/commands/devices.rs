//! Audio device enumeration.

use attune_core::audio::{list_input_devices as core_list_input_devices, DeviceInfo};
use tracing::debug;

/// Enumerate input audio devices visible to the system.
///
/// cpal's device enumeration is synchronous and can take tens of ms on
/// macOS depending on the audio configuration, so we hop onto a
/// blocking task to keep the Tauri command runtime free.
#[tauri::command]
pub async fn list_input_devices() -> Result<Vec<DeviceInfo>, String> {
    debug!("list_input_devices");
    tauri::async_runtime::spawn_blocking(core_list_input_devices)
        .await
        .map_err(|e| format!("list_input_devices task panicked: {e}"))?
        .map_err(|e| e.to_string())
}
