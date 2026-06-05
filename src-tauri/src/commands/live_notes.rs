use std::path::Path;

use attune_core::live_notes::{parse_lines, render_markdown, RawNoteLine};
use attune_core::storage::atomic_write::{atomic_write, atomic_write_json};
use tauri::State;
use tracing::debug;

use crate::app::AppState;

const NOTES_JSON: &str = "live_notes.json";
const NOTES_MARKDOWN: &str = "live-notes.md";

#[tauri::command]
pub async fn save_live_notes(
    state: State<'_, AppState>,
    session_dir: String,
    lines: Vec<RawNoteLine>,
) -> Result<(), String> {
    let output_dir = state.settings.lock().output_dir.clone();

    let dir = attune_core::paths::canonicalize_under(&output_dir, Path::new(&session_dir))
        .map_err(|e| format!("invalid session directory: {e}"))?;
    let markdown = render_markdown(&parse_lines(&lines));

    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        atomic_write_json(&dir.join(NOTES_JSON), &lines).map_err(|e| e.to_string())?;
        atomic_write(&dir.join(NOTES_MARKDOWN), markdown.as_bytes()).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| format!("save_live_notes task panicked: {e}"))??;

    debug!(session_dir, "live notes saved");
    Ok(())
}

#[tauri::command]
pub async fn load_live_notes(
    state: State<'_, AppState>,
    session_dir: String,
) -> Result<Vec<RawNoteLine>, String> {
    let output_dir = state.settings.lock().output_dir.clone();

    let dir = match attune_core::paths::canonicalize_under(&output_dir, Path::new(&session_dir)) {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };
    let path = dir.join(NOTES_JSON);
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<RawNoteLine>, String> {
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e.to_string()),
        }
    })
    .await
    .map_err(|e| format!("load_live_notes task panicked: {e}"))?
}
