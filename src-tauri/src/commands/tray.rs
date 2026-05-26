//! Tray-state updater bridge from the React side. v2 finding 006 /
//! GET-25.
//!
//! The recording store ticks the elapsed counter every second; this
//! command lets it push the current value to the menu bar tray so
//! the title updates live.

use crate::app::tray;

#[tauri::command]
pub fn set_tray_recording(app: tauri::AppHandle, elapsed_secs: Option<u64>) {
    tray::set_recording_state(&app, elapsed_secs);
}
