//! Floating recording-control bar.
//!
//! A compact, frameless, always-on-top window that appears while a
//! capture is in progress so the user always has a recording indicator +
//! a Stop button on hand, no matter which app is focused. Replaces the
//! menu-bar-title-only affordance (the tray still updates too).
//!
//! The bar is draggable (the user parks it wherever), polls
//! `recording_status` for the live elapsed/paused state, and routes its
//! Stop through the main window's recording store (via the
//! `recording-bar:stop` event) so the normal post-stop chain —
//! auto-transcribe, toasts, tray reset — all fire exactly as if the user
//! hit Stop in the app.

use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// Tauri window label for the floating recording bar.
pub const RECORDING_BAR_LABEL: &str = "recording-bar";
/// Label of the app's primary window.
const MAIN_WINDOW_LABEL: &str = "main";
/// Event the main window listens for to run its stop flow.
const STOP_EVENT: &str = "recording-bar:stop";

const BAR_W: f64 = 248.0;
const BAR_H: f64 = 56.0;
const MARGIN: f64 = 24.0;

/// Create (or reveal) the floating recording bar, parked bottom-centre of
/// the active monitor. Never steals focus. Idempotent.
#[tauri::command]
pub fn show_recording_bar(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(RECORDING_BAR_LABEL) {
        let _ = existing.show();
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        &app,
        RECORDING_BAR_LABEL,
        WebviewUrl::App("index.html#/recording-bar".into()),
    )
    .title("Recording")
    .inner_size(BAR_W, BAR_H)
    .resizable(false)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .visible(true)
    .build()
    .map_err(|e| e.to_string())?;

    // Park bottom-centre of the monitor it landed on.
    if let Ok(Some(monitor)) = window.current_monitor() {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let pos = monitor.position();
        let logical_w = size.width as f64 / scale;
        let logical_h = size.height as f64 / scale;
        let x = pos.x as f64 / scale + (logical_w - BAR_W) / 2.0;
        let y = pos.y as f64 / scale + logical_h - BAR_H - MARGIN * 2.0;
        let _ = window.set_position(tauri::LogicalPosition::new(x, y));
    }

    Ok(())
}

/// Close the floating recording bar. No-op when it isn't open.
#[tauri::command]
pub fn hide_recording_bar(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(RECORDING_BAR_LABEL) {
        let _ = window.close();
    }
    Ok(())
}

/// Stop from the bar: tell the main window to run its stop flow. Routed
/// as an event (not a direct backend stop) so the recording store's
/// auto-transcribe + toast + tray-reset chain fires.
#[tauri::command]
pub fn recording_bar_stop(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(main) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = main.emit(STOP_EVENT, ());
    } else {
        let _ = app.emit(STOP_EVENT, ());
    }
    Ok(())
}
