//! Library: enumerate, reveal, and delete saved recording sessions.

use std::path::PathBuf;

use attune_core::storage::{scan_recordings, RecordingSummary};
use tauri::State;
#[cfg(not(target_os = "macos"))]
use tracing::warn;
use tracing::{debug, info};

use crate::app::AppState;

#[tauri::command]
pub fn list_recordings(state: State<'_, AppState>) -> Vec<RecordingSummary> {
    debug!("list_recordings");
    let output_dir = state.settings.lock().output_dir.clone();
    scan_recordings(&output_dir)
}

/// Delete a recording session directory.
///
/// Refuses to delete unless the path lies under the user's configured
/// recordings folder — defence in depth so a bug in the frontend can't
/// trigger an `rm -rf /` situation.
#[tauri::command]
pub fn delete_recording(state: State<'_, AppState>, session_dir: PathBuf) -> Result<(), String> {
    let output_dir = state.settings.lock().output_dir.clone();
    let canon_root = std::fs::canonicalize(&output_dir).map_err(|e| {
        format!(
            "could not canonicalize recordings dir {}: {e}",
            output_dir.display()
        )
    })?;
    let canon_target = std::fs::canonicalize(&session_dir).map_err(|e| {
        format!(
            "could not canonicalize session dir {}: {e}",
            session_dir.display()
        )
    })?;
    if !canon_target.starts_with(&canon_root) {
        return Err(format!(
            "refused to delete {}: not under recordings folder {}",
            canon_target.display(),
            canon_root.display(),
        ));
    }
    if canon_target == canon_root {
        return Err("refused to delete the recordings folder itself".into());
    }
    std::fs::remove_dir_all(&canon_target)
        .map_err(|e| format!("could not delete {}: {e}", canon_target.display()))?;
    info!(path = %canon_target.display(), "recording deleted");
    Ok(())
}

#[tauri::command]
pub fn reveal_in_finder(path: PathBuf) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        warn!("reveal_in_finder not implemented on this platform");
        Ok(())
    }
}
