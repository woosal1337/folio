//! Live note-taking persistence. GET-145.
//!
//! The recording route's editor autosaves the user's anchored notes
//! into the session directory while capture runs. We persist the
//! structured `live_notes.json` (the source of truth the post-meeting
//! pipeline reads) and render the grouped `live-notes.md` next to it so
//! the saved note shows Action items / Decisions / Open questions /
//! Highlights / Notes without a second pass.

use std::path::PathBuf;

use attune_core::live_notes::{parse_lines, render_markdown, RawNoteLine};
use attune_core::storage::atomic_write::{atomic_write, atomic_write_json};
use tracing::debug;

const NOTES_JSON: &str = "live_notes.json";
const NOTES_MARKDOWN: &str = "live-notes.md";

/// Persist the anchored live-notes buffer for a session. Atomic on
/// disk: writes the lossless raw lines as JSON (for editor round-trip)
/// plus the grouped markdown render (the saved note).
#[tauri::command]
pub async fn save_live_notes(session_dir: String, lines: Vec<RawNoteLine>) -> Result<(), String> {
    let dir = PathBuf::from(&session_dir);
    if !dir.is_dir() {
        return Err(format!("session directory does not exist: {session_dir}"));
    }
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

/// Load the raw live-notes lines for a session. Returns an empty vec
/// when the session has no notes yet (fresh recording or resume).
#[tauri::command]
pub async fn load_live_notes(session_dir: String) -> Result<Vec<RawNoteLine>, String> {
    let path = PathBuf::from(&session_dir).join(NOTES_JSON);
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
