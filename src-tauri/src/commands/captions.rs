//! Borderless always-on-top captions window. v2 finding 103 / GET-115.
//!
//! The frontend renders `/captions` inside the existing React app;
//! this command spawns a second Tauri WebView pointed at the same
//! bundle so the captions stay alive across cmd-tab without bringing
//! the main window forward.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tracing::warn;

const CAPTIONS_WINDOW_LABEL: &str = "captions";

/// Open (or focus) the captions window. Idempotent — calling it
/// twice doesn't open a second window. The window is borderless,
/// always-on-top, transparent-friendly, and routed at `/captions`.
#[tauri::command]
pub async fn open_captions_window(app: AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(CAPTIONS_WINDOW_LABEL) {
        existing.show().map_err(|e| e.to_string())?;
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    let window = WebviewWindowBuilder::new(
        &app,
        CAPTIONS_WINDOW_LABEL,
        WebviewUrl::App("index.html#/captions".into()),
    )
    .title("Attune captions")
    .inner_size(720.0, 180.0)
    .min_inner_size(360.0, 120.0)
    .resizable(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(false)
    .build()
    .map_err(|e| format!("could not open captions window: {e}"))?;

    // Best-effort: float across spaces so cmd-tab doesn't park the
    // captions on the user's last desktop.
    #[cfg(target_os = "macos")]
    {
        if let Err(e) = window.set_visible_on_all_workspaces(true) {
            warn!(error = %e, "set_visible_on_all_workspaces failed");
        }
    }
    Ok(())
}

/// Close the captions window if it's open. v2 finding 103 / GET-115.
#[tauri::command]
pub async fn close_captions_window(app: AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(CAPTIONS_WINDOW_LABEL) {
        existing.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}
