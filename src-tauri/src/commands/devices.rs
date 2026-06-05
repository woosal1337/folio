use folio_core::audio::devices::{
    list_input_devices as core_list_input_devices, sample_mic_level, DeviceInfo, MicLevelResult,
};
use folio_core::audio::mic_monitor::MicMonitor;
use tauri::State;
use tracing::{debug, info};

use crate::app::AppState;

#[tauri::command]
pub async fn list_input_devices() -> Result<Vec<DeviceInfo>, String> {
    debug!("list_input_devices");
    tauri::async_runtime::spawn_blocking(core_list_input_devices)
        .await
        .map_err(|e| format!("list_input_devices task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_mic_level(device_name: Option<String>) -> Result<MicLevelResult, String> {
    debug!("check_mic_level");
    tauri::async_runtime::spawn_blocking(move || sample_mic_level(device_name.as_deref(), 500))
        .await
        .map_err(|e| format!("check_mic_level task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_mic_monitor(
    state: State<'_, AppState>,
    device_name: Option<String>,
) -> Result<(), String> {
    info!("start_mic_monitor");
    let monitor = MicMonitor::start(device_name.as_deref()).map_err(|e| e.to_string())?;
    let mut slot = state.mic_monitor.lock();

    if let Some(prev) = slot.take() {
        prev.stop();
    }
    *slot = Some(monitor);
    Ok(())
}

#[tauri::command]
pub fn stop_mic_monitor(state: State<'_, AppState>) {
    info!("stop_mic_monitor");
    if let Some(monitor) = state.mic_monitor.lock().take() {
        monitor.stop();
    }
}
