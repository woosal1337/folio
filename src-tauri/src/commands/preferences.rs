//! Real Preferences NSWindow (replaces the in-app modal).
//! v2 finding 020 / GET-86. Subsumes R10 / GET-116.
//!
//! Opens a separate 640×520 native window pointed at the React route
//! `/preferences-window`. macOS chrome handles the window controls
//! (close / minimize / zoom) so the Cmd-, surface feels like System
//! Settings rather than a Tauri modal.

use tauri::{LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder};

const PREFERENCES_WINDOW_LABEL: &str = "preferences";

#[tauri::command]
pub fn open_preferences_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(PREFERENCES_WINDOW_LABEL) {
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(
        &app,
        PREFERENCES_WINDOW_LABEL,
        WebviewUrl::App("index.html#/preferences-window".into()),
    )
    .title("Attune Preferences")
    .inner_size(640.0, 520.0)
    .min_inner_size(560.0, 420.0)
    .resizable(true)
    .build()
    .map_err(|e| e.to_string())?;
    if let Some(window) = app.get_webview_window(PREFERENCES_WINDOW_LABEL) {
        let _ = window.set_size(LogicalSize::new(640.0, 520.0));
        let _ = window.set_focus();
    }
    Ok(())
}
