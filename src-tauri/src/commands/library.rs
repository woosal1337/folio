//! Library: enumerate, reveal, and delete saved recording sessions.

use std::path::PathBuf;

use attune_core::storage::{scan_recordings, RecordingSummary};
use tauri::State;
#[cfg(not(target_os = "macos"))]
use tracing::warn;
use tracing::{debug, info};

use crate::app::AppState;

/// Scan the user's recordings directory and return a summary per session.
///
/// File-system scan + WAV header reads can take a measurable amount of
/// time once there are many recordings, so this runs on a blocking task.
#[tauri::command]
pub async fn list_recordings(state: State<'_, AppState>) -> Result<Vec<RecordingSummary>, String> {
    debug!("list_recordings");
    let output_dir = state.settings.lock().output_dir.clone();

    tauri::async_runtime::spawn_blocking(move || scan_recordings(&output_dir))
        .await
        .map_err(|e| format!("list_recordings task panicked: {e}"))
}

/// Delete a recording session directory.
///
/// Refuses to delete unless the path lies under the user's configured
/// recordings folder — defence in depth so a bug in the frontend can't
/// trigger an `rm -rf /` situation. The recursive remove runs on a
/// blocking task because removing a session with hundreds of MB of
/// WAVs is not instantaneous.
#[tauri::command]
pub async fn delete_recording(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<(), String> {
    let output_dir = state.settings.lock().output_dir.clone();

    tauri::async_runtime::spawn_blocking(move || -> Result<PathBuf, String> {
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
        Ok(canon_target)
    })
    .await
    .map_err(|e| format!("delete_recording task panicked: {e}"))?
    .map(|path| {
        info!(path = %path.display(), "recording deleted");
    })
}

/// Reveal `path` in the platform file browser. Subprocess spawn is
/// quick but we run it on a blocking task for consistency with the
/// other library commands.
#[tauri::command]
pub async fn reveal_in_finder(path: PathBuf) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
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
    })
    .await
    .map_err(|e| format!("reveal_in_finder task panicked: {e}"))?
}
