//! "Move Attune aside in meetings" (Settings → Preferences).
//!
//! When the meeting watcher sees a conferencing app appear and the user
//! has opted in (`move_aside_in_meetings`), dock the main window to the
//! left of its monitor and shrink it to at most half the screen so the
//! call has room on the right — you keep typing notes alongside it
//! instead of behind it. The pre-move bounds are captured and restored
//! once every monitored app has gone away (or the user turns the setting
//! off).
//!
//! macOS-only in practice — the watcher that drives it only runs there —
//! but the code is platform-agnostic Tauri so it compiles everywhere.

use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Runtime,
};

/// Label of the app's primary window (the Tauri default when
/// `tauri.conf.json` omits an explicit label).
const MAIN_WINDOW_LABEL: &str = "main";
/// Gap from the monitor edges, in logical px.
const MARGIN: f64 = 24.0;
/// Vertical clearance for the macOS menu bar, in logical px.
const MENU_BAR: f64 = 28.0;
/// Don't shrink the window below this height when docking aside.
const MIN_HEIGHT: f64 = 400.0;

/// Window bounds captured before a move, so the user's layout can be
/// restored when the meeting ends.
#[derive(Debug, Clone, Copy)]
pub struct SavedBounds {
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
}

/// Dock the main window to the left of its current monitor, returning the
/// pre-move bounds for a later [`restore`]. Returns `None` (no-op) when
/// the main window or its monitor can't be resolved — every fallible
/// step is best-effort so a positioning hiccup never crashes the watcher.
pub fn move_aside<R: Runtime>(app: &AppHandle<R>) -> Option<SavedBounds> {
    let window = app.get_webview_window(MAIN_WINDOW_LABEL)?;
    let saved = SavedBounds {
        position: window.outer_position().ok()?,
        size: window.inner_size().ok()?,
    };

    let monitor = window.current_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    let mon_pos = monitor.position();
    let mon_size = monitor.size();
    let mon_left = mon_pos.x as f64 / scale;
    let mon_top = mon_pos.y as f64 / scale;
    let mon_w = mon_size.width as f64 / scale;
    let mon_h = mon_size.height as f64 / scale;

    // Target width: the current width, but never more than half the
    // monitor (less margins) so the conferencing app has room beside us.
    let cur_w = saved.size.width as f64 / scale;
    let target_w = cur_w.min((mon_w / 2.0) - MARGIN * 1.5).max(360.0);
    let target_h = (mon_h - MARGIN * 2.0 - MENU_BAR).max(MIN_HEIGHT);

    let _ = window.set_size(LogicalSize::new(target_w, target_h));
    let _ = window.set_position(LogicalPosition::new(
        mon_left + MARGIN,
        mon_top + MARGIN + MENU_BAR,
    ));
    Some(saved)
}

/// Restore the bounds captured by [`move_aside`]. Best-effort.
pub fn restore<R: Runtime>(app: &AppHandle<R>, bounds: SavedBounds) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.set_size(bounds.size);
        let _ = window.set_position(bounds.position);
    }
}
