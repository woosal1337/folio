use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const RECORDING_BAR_LABEL: &str = "recording-bar";

const MAIN_WINDOW_LABEL: &str = "main";

const STOP_EVENT: &str = "recording-bar:stop";

const PAUSE_EVENT: &str = "recording-bar:pause";
const RESUME_EVENT: &str = "recording-bar:resume";

const BAR_W: f64 = 46.0;
const BAR_H: f64 = 196.0;
const MARGIN: f64 = 24.0;

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
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .visible(true)
    .build()
    .map_err(|e| e.to_string())?;

    if let Ok(Some(monitor)) = window.current_monitor() {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let pos = monitor.position();
        let logical_w = size.width as f64 / scale;
        let logical_h = size.height as f64 / scale;
        let x = pos.x as f64 / scale + logical_w - BAR_W - MARGIN;
        let y = pos.y as f64 / scale + (logical_h - BAR_H) / 2.0;
        let _ = window.set_position(tauri::LogicalPosition::new(x, y));
    }

    Ok(())
}

#[tauri::command]
pub fn hide_recording_bar(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(RECORDING_BAR_LABEL) {
        let _ = window.hide();
    }
    Ok(())
}

#[tauri::command]
pub fn recording_bar_stop(app: tauri::AppHandle) -> Result<(), String> {
    emit_to_main(&app, STOP_EVENT);
    Ok(())
}

#[tauri::command]
pub fn recording_bar_pause(app: tauri::AppHandle) -> Result<(), String> {
    emit_to_main(&app, PAUSE_EVENT);
    Ok(())
}

#[tauri::command]
pub fn recording_bar_resume(app: tauri::AppHandle) -> Result<(), String> {
    emit_to_main(&app, RESUME_EVENT);
    Ok(())
}

fn emit_to_main(app: &tauri::AppHandle, event: &str) {
    if let Some(main) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = main.emit(event, ());
    } else {
        let _ = app.emit(event, ());
    }
}
