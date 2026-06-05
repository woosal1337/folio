use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Runtime,
};

const MAIN_WINDOW_LABEL: &str = "main";

const MARGIN: f64 = 24.0;

const MENU_BAR: f64 = 28.0;

const MIN_HEIGHT: f64 = 400.0;

#[derive(Debug, Clone, Copy)]
pub struct SavedBounds {
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
}

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

pub fn restore<R: Runtime>(app: &AppHandle<R>, bounds: SavedBounds) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.set_size(bounds.size);
        let _ = window.set_position(bounds.position);
    }
}
