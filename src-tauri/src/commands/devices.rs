//! Audio device enumeration.

use attune_core::audio::{list_input_devices as core_list_input_devices, DeviceInfo};
use tracing::debug;

#[tauri::command]
pub fn list_input_devices() -> Result<Vec<DeviceInfo>, String> {
    debug!("list_input_devices");
    core_list_input_devices().map_err(|e| e.to_string())
}
