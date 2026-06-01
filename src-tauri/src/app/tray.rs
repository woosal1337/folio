//! Menu bar (system tray) integration. v2 finding 006 / GET-25.
//!
//! Top-priority hero feature — cited by 8 lenses. Recording must be
//! ambient because users live in Zoom, not in Attune. The tray
//! surface:
//!
//!   * Always-on menu bar icon: distinct glyphs for idle, recording,
//!     paused, and Privacy Mode (airgapped). Idle/paused/airgapped
//!     use macOS template images so they adapt to light/dark menu
//!     bars; the recording glyph is a red filled circle (not a
//!     template) so it stands out (GET-201).
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

use tauri::image::Image as TrayImage;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Runtime};

const TRAY_ID: &str = "attune-menubar";
const MENU_START: &str = "start_recording";
const MENU_STOP: &str = "stop_recording";
const MENU_OPEN: &str = "open_attune";
const MENU_INBOX: &str = "open_inbox";
const MENU_QUIT: &str = "quit_attune";

// ---------------------------------------------------------------------------
// Tray icon bitmaps (GET-201)
//
// Each icon is a 22×22 RGBA image (1936 bytes). White pixels on a
// transparent background are treated as macOS template images
// (auto-adapt to light/dark menu bar). The recording icon uses
// red pixels so it stands out in both appearances.
// ---------------------------------------------------------------------------

const ICON_SIZE: u32 = 22;

/// Fill a 22×22 RGBA buffer with all-transparent pixels.
fn blank() -> Vec<u8> {
    vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize]
}

/// Write a filled rectangle into the RGBA buffer (clipped to bounds).
#[allow(clippy::too_many_arguments)]
fn fill_rect(buf: &mut [u8], x: u32, y: u32, w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) {
    for row in y..y.saturating_add(h).min(ICON_SIZE) {
        for col in x..x.saturating_add(w).min(ICON_SIZE) {
            let idx = ((row * ICON_SIZE + col) * 4) as usize;
            buf[idx] = r;
            buf[idx + 1] = g;
            buf[idx + 2] = b;
            buf[idx + 3] = a;
        }
    }
}

/// Write a filled circle into the RGBA buffer.
#[allow(clippy::too_many_arguments)]
fn fill_circle(buf: &mut [u8], cx: u32, cy: u32, radius: u32, r: u8, g: u8, b: u8, a: u8) {
    let r2 = (radius * radius) as i64;
    for row in 0..ICON_SIZE {
        for col in 0..ICON_SIZE {
            let dx = col as i64 - cx as i64;
            let dy = row as i64 - cy as i64;
            if dx * dx + dy * dy <= r2 {
                let idx = ((row * ICON_SIZE + col) * 4) as usize;
                buf[idx] = r;
                buf[idx + 1] = g;
                buf[idx + 2] = b;
                buf[idx + 3] = a;
            }
        }
    }
}

/// Idle icon: waveform — 3 vertical bars at different heights.
/// Monochrome white (template image).
fn idle_icon_rgba() -> Vec<u8> {
    let mut buf = blank();
    // Bar heights: 8, 14, 8 px; bars are 3px wide with 2px gaps.
    // Centered horizontally in 22px: total width = 3+2+3+2+3 = 13, left = (22-13)/2 = 4
    fill_rect(&mut buf, 4, 11, 3, 8, 255, 255, 255, 255); // left bar: y=11..19
    fill_rect(&mut buf, 9, 7, 3, 14, 255, 255, 255, 255); // center bar: y=7..21
    fill_rect(&mut buf, 14, 11, 3, 8, 255, 255, 255, 255); // right bar: y=11..19
    buf
}

/// Recording icon: filled red circle.
/// Not a template image — uses red so it stands out in both light/dark.
fn recording_icon_rgba() -> Vec<u8> {
    let mut buf = blank();
    // Filled circle at center, radius 8px.
    fill_circle(&mut buf, 11, 11, 8, 220, 38, 38, 255); // red-600
    buf
}

/// Paused icon: two vertical bars (pause symbol). Monochrome white.
fn paused_icon_rgba() -> Vec<u8> {
    let mut buf = blank();
    // Two bars: 5px wide, 12px tall, centered.
    // Total width = 5+4+5 = 14, left = (22-14)/2 = 4
    fill_rect(&mut buf, 4, 5, 5, 12, 255, 255, 255, 255);
    fill_rect(&mut buf, 13, 5, 5, 12, 255, 255, 255, 255);
    buf
}

/// Airgapped icon: lock body outline (circle top + rectangle bottom).
/// Monochrome white (template image).
fn airgap_icon_rgba() -> Vec<u8> {
    let mut buf = blank();
    // Lock shackle (arc): draw as a partial circle outline at top.
    // Body: a rounded rectangle in the lower half.
    // Simple approximation: circle outline (radius 4, center 11, 9) for shackle.
    let cx = 11u32;
    let cy = 9u32;
    let outer = 4u32;
    let inner = 2u32;
    for row in 0..ICON_SIZE {
        for col in 0..ICON_SIZE {
            let dx = col as i64 - cx as i64;
            let dy = row as i64 - cy as i64;
            let d2 = dx * dx + dy * dy;
            let in_ring =
                d2 <= (outer * outer) as i64 && d2 >= (inner * inner) as i64 && row < cy + 2;
            if in_ring {
                let idx = ((row * ICON_SIZE + col) * 4) as usize;
                buf[idx] = 255;
                buf[idx + 1] = 255;
                buf[idx + 2] = 255;
                buf[idx + 3] = 255;
            }
        }
    }
    // Lock body (filled rectangle).
    fill_rect(&mut buf, 5, 11, 12, 9, 255, 255, 255, 255);
    // Keyhole (transparent circle in the body).
    fill_circle(&mut buf, 11, 15, 2, 0, 0, 0, 0);
    buf
}

fn make_image(rgba: Vec<u8>) -> TrayImage<'static> {
    TrayImage::new_owned(rgba, ICON_SIZE, ICON_SIZE)
}

// ---------------------------------------------------------------------------
// Tray install
// ---------------------------------------------------------------------------

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
        .icon(make_image(idle_icon_rgba()))
        .icon_as_template(true)
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

// ---------------------------------------------------------------------------
// Recording state (GET-201)
// ---------------------------------------------------------------------------

/// Recording state passed to [`set_recording_state`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    /// No active recording.
    Idle,
    /// Capturing — `elapsed_secs` is the recording duration.
    Recording(u64),
    /// Capture paused.
    Paused(u64),
    /// Privacy Mode (airgap) on — all egress blocked.
    Airgapped,
}

/// Update the tray tooltip, title, and icon glyph. Idempotent.
///
/// Call signature mirrors the legacy `set_recording_state(app, elapsed)` but
/// now also accepts paused / airgapped states via [`TrayState`].
pub fn set_tray_state<R: Runtime>(app: &AppHandle<R>, state: TrayState) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let (tooltip, title, rgba, is_template) = match state {
        TrayState::Idle => ("Attune — idle".to_string(), None, idle_icon_rgba(), true),
        TrayState::Recording(secs) => (
            format!("Attune — recording {}", format_elapsed(secs)),
            Some(format!("● {}", format_elapsed(secs))),
            recording_icon_rgba(),
            false, // red icon — not a template
        ),
        TrayState::Paused(secs) => (
            format!("Attune — paused {}", format_elapsed(secs)),
            Some(format!("⏸ {}", format_elapsed(secs))),
            paused_icon_rgba(),
            true,
        ),
        TrayState::Airgapped => (
            "Attune — Privacy Mode on".to_string(),
            Some("🔒".to_string()),
            airgap_icon_rgba(),
            true,
        ),
    };

    let _ = tray.set_tooltip(Some(&tooltip));
    let _ = tray.set_title(title.as_deref());

    let _ = tray.set_icon(Some(make_image(rgba)));
    let _ = tray.set_icon_as_template(is_template);

    // Update the Dock badge (GET-201).
    set_dock_badge(matches!(state, TrayState::Recording(_)));
}

/// Set or clear the Dock badge while recording. Shows a red dot (●)
/// when `visible`, clears it when not. Uses ObjC to set the badge
/// label on the app's dock tile.
#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn set_dock_badge(visible: bool) {
    use std::ffi::CString;

    // SAFETY: ObjC runtime calls on AppKit objects. NSApp and dockTile
    // are stable singleton accessors; badge label is a string copy (nil
    // clears it). Runs on the main thread via the tray event loop.
    unsafe {
        use cocoa::base::{id, nil};
        use objc::{class, msg_send, sel, sel_impl};

        let app: id = msg_send![class!(NSApplication), sharedApplication];
        if app == nil {
            return;
        }
        let dock_tile: id = msg_send![app, dockTile];
        if dock_tile == nil {
            return;
        }
        if visible {
            // Bind to a variable so the CString lives past the as_ptr() call.
            let label_c = CString::new("●").unwrap_or_default();
            let ns_str: id = msg_send![class!(NSString), stringWithUTF8String: label_c.as_ptr()];
            let _: () = msg_send![dock_tile, setBadgeLabel: ns_str];
        } else {
            let _: () = msg_send![dock_tile, setBadgeLabel: nil];
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn set_dock_badge(_visible: bool) {}

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
    use super::*;

    #[test]
    fn format_elapsed_renders_short_and_long_durations() {
        assert_eq!(format_elapsed(0), "0:00");
        assert_eq!(format_elapsed(42), "0:42");
        assert_eq!(format_elapsed(125), "2:05");
        assert_eq!(format_elapsed(3600), "1:00:00");
        assert_eq!(format_elapsed(3725), "1:02:05");
    }

    #[test]
    fn icon_rgba_buffers_are_correct_size() {
        let expected = (ICON_SIZE * ICON_SIZE * 4) as usize;
        assert_eq!(idle_icon_rgba().len(), expected);
        assert_eq!(recording_icon_rgba().len(), expected);
        assert_eq!(paused_icon_rgba().len(), expected);
        assert_eq!(airgap_icon_rgba().len(), expected);
    }

    #[test]
    fn recording_icon_has_red_pixels() {
        let buf = recording_icon_rgba();
        // Center pixel should be red.
        let cx = 11usize;
        let cy = 11usize;
        let idx = (cy * ICON_SIZE as usize + cx) * 4;
        assert_eq!(buf[idx], 220, "R channel should be red-600");
        assert!(buf[idx + 3] > 0, "alpha should be non-zero");
    }

    #[test]
    fn idle_icon_has_white_pixels() {
        let buf = idle_icon_rgba();
        // Center-top of the tall middle bar should be white.
        let col = 10usize; // center of middle bar (cols 9-11)
        let row = 9usize; // inside the tall bar
        let idx = (row * ICON_SIZE as usize + col) * 4;
        assert_eq!(buf[idx], 255, "idle icon should have white pixels");
    }
}
