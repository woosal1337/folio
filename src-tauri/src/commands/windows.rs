//! Multi-window commands. v2 finding 014 / GET-48.
//!
//! Cmd-N opens a fresh Record window. Cmd-Shift-L opens a standalone
//! Library window. Double-clicking a Library row opens that
//! recording in its own Editor window. Each window's URL fragment
//! determines its route — `#/record`, `#/library`, `#/editor/<label>`.
//!
//! Window labels are derived from the route so re-opening the same
//! editor focuses the existing window instead of stacking duplicates.

use tauri::{AppHandle, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder};

const RECORD_PREFIX: &str = "record-";
const LIBRARY_LABEL: &str = "library-standalone";
const EDITOR_PREFIX: &str = "editor-";

#[tauri::command]
pub fn open_record_window(app: AppHandle) -> Result<(), String> {
    let label = format!("{}{}", RECORD_PREFIX, app.webview_windows().len());
    WebviewWindowBuilder::new(&app, label, WebviewUrl::App("index.html#/record".into()))
        .title("Attune — Record")
        .inner_size(1200.0, 780.0)
        .min_inner_size(880.0, 600.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_library_window(app: AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(LIBRARY_LABEL) {
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(
        &app,
        LIBRARY_LABEL,
        WebviewUrl::App("index.html#/library".into()),
    )
    .title("Attune — Library")
    .inner_size(1100.0, 720.0)
    .min_inner_size(720.0, 540.0)
    .build()
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_editor_window(app: AppHandle, label: String) -> Result<(), String> {
    let safe = sanitise_label(&label);
    let window_label = format!("{EDITOR_PREFIX}{safe}");
    if let Some(existing) = app.get_webview_window(&window_label) {
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }
    let url = format!("index.html#/editor/{}", urlencoding::encode(&label));
    let window = WebviewWindowBuilder::new(&app, window_label, WebviewUrl::App(url.into()))
        .title(format!("Attune — {label}"))
        .inner_size(1000.0, 700.0)
        .min_inner_size(720.0, 540.0)
        .build()
        .map_err(|e| e.to_string())?;
    let _ = window.set_size(LogicalSize::new(1000.0, 700.0));
    Ok(())
}

/// Sanitise a recording label for use as a Tauri window label. Window
/// labels must be ASCII alphanumeric + dash + underscore. We replace
/// every other character with `_` and cap at 80 characters.
pub fn sanitise_label(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut last_underscore = false;
    for ch in label.chars() {
        let safe = if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            ch
        } else {
            '_'
        };
        if safe == '_' {
            if last_underscore {
                continue;
            }
            last_underscore = true;
        } else {
            last_underscore = false;
        }
        out.push(safe);
    }
    out.trim_matches('_').chars().take(80).collect()
}

#[cfg(test)]
mod tests {
    use super::sanitise_label;

    #[test]
    fn sanitise_label_keeps_alphanumeric_and_dash_underscore() {
        assert_eq!(sanitise_label("2026-05-26-meeting"), "2026-05-26-meeting");
        assert_eq!(sanitise_label("alpha_beta-1"), "alpha_beta-1");
    }

    #[test]
    fn sanitise_label_collapses_disallowed_chars_into_single_underscores() {
        let safe = sanitise_label("2026/05/26 / pricing review");
        assert!(!safe.contains('/'));
        assert!(!safe.contains("__"));
        assert!(!safe.starts_with('_'));
        assert!(!safe.ends_with('_'));
    }

    #[test]
    fn sanitise_label_caps_at_80_chars() {
        let long = "a".repeat(200);
        assert!(sanitise_label(&long).len() <= 80);
    }
}
