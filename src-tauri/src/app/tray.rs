use tauri::image::Image as TrayImage;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Runtime};

const TRAY_ID: &str = "folio-menubar";
const MENU_START: &str = "start_recording";
const MENU_STOP: &str = "stop_recording";
const MENU_OPEN: &str = "open_folio";
const MENU_INBOX: &str = "open_inbox";
const MENU_QUIT: &str = "quit_folio";

const ICON_SIZE: u32 = 22;

fn blank() -> Vec<u8> {
    vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize]
}

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

const DOT_CX: u32 = 11;
const DOT_CY: u32 = 11;
const DOT_RADIUS: u32 = 6;

fn idle_icon_rgba() -> Vec<u8> {
    let mut buf = blank();

    fill_circle(&mut buf, DOT_CX, DOT_CY, DOT_RADIUS, 142, 142, 147, 255);
    buf
}

fn recording_icon_rgba() -> Vec<u8> {
    let mut buf = blank();

    fill_circle(&mut buf, DOT_CX, DOT_CY, DOT_RADIUS, 220, 38, 38, 255);
    buf
}

fn paused_icon_rgba() -> Vec<u8> {
    let mut buf = blank();

    fill_circle(&mut buf, DOT_CX, DOT_CY, DOT_RADIUS, 245, 158, 11, 255);
    buf
}

fn airgap_icon_rgba() -> Vec<u8> {
    let mut buf = blank();

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

    fill_rect(&mut buf, 5, 11, 12, 9, 255, 255, 255, 255);

    fill_circle(&mut buf, 11, 15, 2, 0, 0, 0, 0);
    buf
}

fn make_image(rgba: Vec<u8>) -> TrayImage<'static> {
    TrayImage::new_owned(rgba, ICON_SIZE, ICON_SIZE)
}

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
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Folio", true, Some("CmdOrCtrl+Q"))?;

    let menu = Menu::with_items(
        app,
        &[&start, &stop, &separator, &open, &inbox, &separator, &quit],
    )?;

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Folio — idle")
        .icon(make_image(idle_icon_rgba()))
        .icon_as_template(false)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    Idle,

    Recording(u64),

    Paused(u64),

    Airgapped,
}

pub fn set_tray_state<R: Runtime>(app: &AppHandle<R>, state: TrayState) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let (tooltip, title, rgba, is_template) = match state {
        TrayState::Idle => ("Folio — idle".to_string(), None, idle_icon_rgba(), false),
        TrayState::Recording(secs) => (
            format!("Folio — recording {}", format_elapsed(secs)),
            Some(format_elapsed(secs)),
            recording_icon_rgba(),
            false,
        ),
        TrayState::Paused(secs) => (
            format!("Folio — paused {}", format_elapsed(secs)),
            Some(format_elapsed(secs)),
            paused_icon_rgba(),
            false,
        ),
        TrayState::Airgapped => (
            "Folio — Privacy Mode on".to_string(),
            None,
            airgap_icon_rgba(),
            true,
        ),
    };

    let _ = tray.set_tooltip(Some(&tooltip));
    let _ = tray.set_title(title.as_deref());

    let _ = tray.set_icon(Some(make_image(rgba)));
    let _ = tray.set_icon_as_template(is_template);

    set_dock_badge(matches!(state, TrayState::Recording(_)));
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn set_dock_badge(visible: bool) {
    use std::ffi::CString;

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

        let cx = 11usize;
        let cy = 11usize;
        let idx = (cy * ICON_SIZE as usize + cx) * 4;
        assert_eq!(buf[idx], 220, "R channel should be red-600");
        assert!(buf[idx + 3] > 0, "alpha should be non-zero");
    }

    #[test]
    fn idle_icon_has_gray_pixels() {
        let buf = idle_icon_rgba();

        let idx = (DOT_CY as usize * ICON_SIZE as usize + DOT_CX as usize) * 4;
        assert_eq!(buf[idx], 142, "idle icon center should be system gray");
        assert!(buf[idx + 3] > 0, "alpha should be non-zero");
    }
}
