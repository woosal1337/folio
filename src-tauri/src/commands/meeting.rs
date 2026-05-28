//! Meeting-detection HUD commands. GET-143.
//!
//! The HUD window (label `meeting-hud`) is a frameless always-on-top
//! popover the watcher opens when a conferencing app appears. These
//! commands back its three actions — Take Notes, Dismiss, and Don't ask
//! for <App> — plus the initial `get_pending_meeting` read it does on
//! mount.

use attune_core::storage::SettingsStore;
use tauri::{Emitter, Manager, State};
use tracing::info;

use crate::app::meeting_watcher::{DetectedMeeting, MEETING_HUD_LABEL};
use crate::app::AppState;

/// Label of the app's primary window (the implicit Tauri default when
/// `tauri.conf.json` omits an explicit label).
const MAIN_WINDOW_LABEL: &str = "main";
/// Event the main window listens for to start the one-click capture flow.
const TAKE_NOTES_EVENT: &str = "meeting:take-notes";

/// The meeting currently awaiting a decision in the HUD, if any. The HUD
/// calls this on mount; subsequent detections arrive via the
/// `meeting-detected` event.
#[tauri::command]
pub fn get_pending_meeting(state: State<'_, AppState>) -> Option<DetectedMeeting> {
    state.pending_meeting.lock().clone()
}

/// Take Notes: bring the main window forward and tell it to start
/// capturing, then close the HUD. Recording lives in the global backend
/// session, but we route through the main window's recording store so
/// its ticker, tray updates, and auto-transcribe-on-stop chain all fire.
#[tauri::command]
pub fn meeting_take_notes(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    *state.pending_meeting.lock() = None;

    if let Some(main) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = main.show();
        let _ = main.unminimize();
        let _ = main.set_focus();
        let _ = main.emit(TAKE_NOTES_EVENT, ());
    } else {
        // No main window listening — emit app-wide as a fallback.
        let _ = app.emit(TAKE_NOTES_EVENT, ());
    }

    close_hud(&app);
    info!("meeting take-notes triggered from HUD");
    Ok(())
}

/// Dismiss: drop the pending meeting and close the HUD. Leaves the
/// per-app mute list untouched, so the next call still surfaces.
#[tauri::command]
pub fn dismiss_meeting_hud(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    *state.pending_meeting.lock() = None;
    close_hud(&app);
    Ok(())
}

/// Don't ask for <App>: add the bundle id to the muted list, persist it,
/// and close the HUD. The watcher reads the muted list each tick, so the
/// suppression takes effect immediately.
#[tauri::command]
pub async fn suppress_meeting_app(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    bundle_id: String,
) -> Result<(), String> {
    let (settings_to_save, path) = {
        let mut s = state.settings.lock();
        if !s.notification_muted_apps.iter().any(|m| m == &bundle_id) {
            s.notification_muted_apps.push(bundle_id.clone());
        }
        (s.clone(), state.settings_store.path().to_path_buf())
    };
    *state.pending_meeting.lock() = None;

    let to_persist = settings_to_save.clone();
    tauri::async_runtime::spawn_blocking(move || SettingsStore::new(path).save(&to_persist))
        .await
        .map_err(|e| format!("suppress_meeting_app task panicked: {e}"))?
        .map_err(|e| e.to_string())?;

    *state.settings.lock() = settings_to_save;
    close_hud(&app);
    info!(bundle_id, "muted auto-detect for app from HUD");
    Ok(())
}

fn close_hud(app: &tauri::AppHandle) {
    if let Some(hud) = app.get_webview_window(MEETING_HUD_LABEL) {
        let _ = hud.close();
    }
}
