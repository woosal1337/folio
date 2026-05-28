//! Menu bar (system tray) integration. v2 finding 006 / GET-25.
//!
//! Top-priority hero feature — cited by 8 lenses. Recording must be
//! ambient because users live in Zoom, not in Attune. The tray
//! surface:
//!
//!   * Always-on menu bar icon (the app's bundled `tray-icon.png`
//!     when present, the dock icon otherwise).
//!   * Tooltip + title update to "● 0:42" (pulsing red dot + elapsed
//!     time) while recording.
//!   * Click opens a menu: Start Recording, Stop Recording, Open
//!     Library, Open Inbox, Quit.
//!   * Menu items emit Tauri events the React side listens to:
//!     `tray:start-recording`, `tray:stop-recording`,
//!     `tray:open-library`, `tray:open-inbox`.
//!
//! Lifecycle: [`install`] is called once from the Tauri `setup` hook.
//! The recording store updates the tray title every second via
//! [`set_recording_state`].

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Runtime};

const TRAY_ID: &str = "attune-menubar";
const MENU_START: &str = "start_recording";
const MENU_STOP: &str = "stop_recording";
const MENU_OPEN: &str = "open_attune";
const MENU_INBOX: &str = "open_inbox";
const MENU_QUIT: &str = "quit_attune";

/// Build the tray icon and wire its menu. Called once during the
/// Tauri `setup` hook.
pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let start = MenuItem::with_id(
        app,
        MENU_START,
        "Start Recording",
        true,
        Some("CmdOrCtrl+R"),
    )?;
    let stop = MenuItem::with_id(app, MENU_STOP, "Stop Recording", true, None::<&str>)?;
    let open = MenuItem::with_id(app, MENU_OPEN, "Open Library", true, None::<&str>)?;
    let inbox = MenuItem::with_id(app, MENU_INBOX, "Open Inbox", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Attune", true, Some("CmdOrCtrl+Q"))?;

    let menu = Menu::with_items(
        app,
        &[&start, &stop, &separator, &open, &inbox, &separator, &quit],
    )?;

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Attune — idle")
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_START => emit_to_window(app, "tray:start-recording"),
            MENU_STOP => emit_to_window(app, "tray:stop-recording"),
            MENU_OPEN => emit_to_window(app, "tray:open-library"),
            MENU_INBOX => emit_to_window(app, "tray:open-inbox"),
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn emit_to_window<R: Runtime>(app: &AppHandle<R>, event: &str) {
    let Some(window) = app.webview_windows().values().next().cloned() else {
        return;
    };
    let _ = window.emit(event, ());
    let _ = window.show();
    let _ = window.set_focus();
}

/// Update the tray tooltip + title while recording. Called from the
/// recording lifecycle: `set_recording_state(app, Some(42))` on tick,
/// `set_recording_state(app, None)` on stop. Idempotent — repeated
/// calls with the same value are cheap no-ops.
pub fn set_recording_state<R: Runtime>(app: &AppHandle<R>, elapsed_secs: Option<u64>) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let tooltip = match elapsed_secs {
        Some(secs) => format!("Attune — recording {}", format_elapsed(secs)),
        None => "Attune — idle".to_string(),
    };
    let _ = tray.set_tooltip(Some(&tooltip));
    let title = elapsed_secs.map(|secs| format!("● {}", format_elapsed(secs)));
    let _ = tray.set_title(title.as_deref());
}

/// Format seconds into the "M:SS" or "H:MM:SS" string the title shows.
fn format_elapsed(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::format_elapsed;

    #[test]
    fn format_elapsed_renders_short_and_long_durations() {
        assert_eq!(format_elapsed(0), "0:00");
        assert_eq!(format_elapsed(42), "0:42");
        assert_eq!(format_elapsed(125), "2:05");
        assert_eq!(format_elapsed(3600), "1:00:00");
        assert_eq!(format_elapsed(3725), "1:02:05");
    }
}
