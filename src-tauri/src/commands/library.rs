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
///
/// Filters out the currently-active recording session. The capture
/// pipeline creates the session dir and the mic.wav / system.wav files
/// up-front, but the WAV headers are not finalized until
/// [`CaptureSession::stop`] runs — so if we surface that directory in
/// the list while a recording is in progress, the `<audio>` element on
/// the frontend hits MediaError 4 ("source not supported"). Hiding it
/// until stop keeps the library honest about what is actually playable.
#[tauri::command]
pub async fn list_recordings(state: State<'_, AppState>) -> Result<Vec<RecordingSummary>, String> {
    debug!("list_recordings");
    let output_dir = state.settings.lock().output_dir.clone();
    let active_session_dir = state
        .session
        .lock()
        .as_ref()
        .map(|s| s.session_dir().clone());

    tauri::async_runtime::spawn_blocking(move || {
        let mut list = scan_recordings(&output_dir);
        if let Some(active) = active_session_dir {
            list.retain(|entry| entry.session_dir != active);
        }
        list
    })
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

/// Look up a single recording by its label (the session directory's
/// timestamp name). Used by the Editor route when the user lands on a
/// `/editor/:label` URL directly and does not have the `RecordingSummary`
/// in router state.
#[tauri::command]
pub async fn get_recording(
    state: State<'_, AppState>,
    label: String,
) -> Result<Option<RecordingSummary>, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    let active_session_dir = state
        .session
        .lock()
        .as_ref()
        .map(|s| s.session_dir().clone());

    tauri::async_runtime::spawn_blocking(move || {
        let list = scan_recordings(&output_dir);
        list.into_iter()
            .find(|r| r.label == label && Some(&r.session_dir) != active_session_dir.as_ref())
    })
    .await
    .map_err(|e| format!("get_recording task panicked: {e}"))
}

/// Reveal `path` in the platform file browser. Subprocess spawn is
/// quick but we run it on a blocking task for consistency with the
/// other library commands.
#[tauri::command]
pub async fn reveal_in_finder(
    state: State<'_, AppState>,
    path: PathBuf,
) -> Result<(), String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let canon = attune_core::paths::canonicalize_under(&output_dir, &path)
            .map_err(|e| e.to_string())?;
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(&canon)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = canon;
            warn!("reveal_in_finder not implemented on this platform");
            Ok(())
        }
    })
    .await
    .map_err(|e| format!("reveal_in_finder task panicked: {e}"))?
}

/// Save a voice-debrief blob next to an existing recording. v2 finding
/// 027 / GET-53. The frontend records mic via MediaRecorder, hands us
/// the raw container bytes (`debrief.webm` by default) and we write
/// them atomically into the session directory.
#[tauri::command]
pub async fn save_debrief(
    state: State<'_, AppState>,
    session_dir: PathBuf,
    filename: String,
    bytes: Vec<u8>,
) -> Result<PathBuf, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<PathBuf, String> {
        let canon_root = std::fs::canonicalize(&output_dir).map_err(|e| e.to_string())?;
        let canon_target = std::fs::canonicalize(&session_dir).map_err(|e| e.to_string())?;
        if !canon_target.starts_with(&canon_root) {
            return Err(format!(
                "refused: {} not under recordings folder",
                canon_target.display()
            ));
        }
        // Filename sanitisation: no path components, no leading dots.
        let safe = filename.replace(['/', '\\'], "_");
        if safe.starts_with('.') || safe.is_empty() {
            return Err(format!("invalid debrief filename: {safe}"));
        }
        let final_path = canon_target.join(&safe);
        let tmp_path = canon_target.join(format!("{safe}.tmp"));
        std::fs::write(&tmp_path, &bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp_path, &final_path).map_err(|e| e.to_string())?;
        info!(
            "save_debrief: wrote {} bytes to {}",
            bytes.len(),
            final_path.display()
        );
        Ok(final_path)
    })
    .await
    .map_err(|e| format!("save_debrief task panicked: {e}"))?
}

/// Present the macOS native share sheet (`NSSharingServicePicker`)
/// anchored to the current key window for one or more files. v2
/// finding 010 / GET-34 — AirDrop, Messages, Mail, Notes, third-party
/// share extensions for free, with zero per-target plumbing.
#[tauri::command]
pub async fn share_paths(
    state: State<'_, AppState>,
    paths: Vec<PathBuf>,
) -> Result<(), String> {
    info!("share_paths: {} item(s)", paths.len());
    let output_dir = state.settings.lock().output_dir.clone();
    let mut canon_paths: Vec<PathBuf> = Vec::with_capacity(paths.len());
    for p in &paths {
        let canon = attune_core::paths::canonicalize_under(&output_dir, p)
            .map_err(|e| e.to_string())?;
        canon_paths.push(canon);
    }
    crate::app::share_sheet::share_paths(&canon_paths)
}
