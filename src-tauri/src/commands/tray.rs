use crate::app::tray::{self, TrayState};

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
