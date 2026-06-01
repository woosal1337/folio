//! Tray-state updater bridge from the React side. v2 finding 006 /
//! GET-25.
//!
//! The recording store ticks the elapsed counter every second; this
//! command lets it push the current value to the menu bar tray so
//! the title updates live. GET-201 extends this to also carry the
//! paused / airgapped flag so the tray icon glyph updates correctly.

use crate::app::tray::{self, TrayState};

/// Update the menu bar tray state.
///
/// - `elapsed_secs = None, paused = false, airgapped = false` → idle
/// - `elapsed_secs = Some(n), paused = false`                 → recording
/// - `elapsed_secs = Some(n), paused = true`                  → paused
/// - `airgapped = true`                                        → airgap glyph
#[tauri::command]
pub fn set_tray_recording(
    app: tauri::AppHandle,
    elapsed_secs: Option<u64>,
    #[allow(unused_variables)] paused: Option<bool>,
    #[allow(unused_variables)] airgapped: Option<bool>,
) {
    let state = if airgapped.unwrap_or(false) {
        TrayState::Airgapped
    } else {
        match elapsed_secs {
            None => TrayState::Idle,
            Some(secs) => {
                if paused.unwrap_or(false) {
                    TrayState::Paused(secs)
                } else {
                    TrayState::Recording(secs)
                }
            }
        }
    };
    tray::set_tray_state(&app, state);
}
