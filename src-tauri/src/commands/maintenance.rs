//! One-off maintenance commands the frontend calls when the user wants
//! to undo work and start over on a recording. Today: clearing
//! transcripts so they can be regenerated against the latest pipeline,
//! plus building a vault-snapshot zip (v2 finding 057 / GET-92).

use std::path::PathBuf;

use attune_core::llm::AgentRunStore;
use attune_core::storage::retention::{purge_old_wavs, PurgeSummary};
use attune_core::storage::snapshot::{
    export as export_snapshot_impl, SnapshotPaths, SnapshotSummary,
};
use tauri::State;
use tracing::info;

use crate::app::AppState;

const TRANSCRIPT_FILENAME: &str = "transcript.json";

/// Delete the transcript and every saved agent run for a recording so
/// that the next transcription pass starts from a clean slate.
///
/// Used by the "Re-transcribe" UX path on legacy transcripts: the
/// frontend calls this, then calls the normal `transcribe_recording`
/// command. Idempotent — succeeds whether anything was actually
/// present to delete.
///
/// Audio files (`mic.wav`, `system.wav`) are intentionally left
/// alone. This command never touches the source recording.
#[tauri::command]
pub async fn clear_recording_artifacts(session_dir: PathBuf) -> Result<(), String> {
    let dir = session_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let transcript_path = dir.join(TRANSCRIPT_FILENAME);
        match std::fs::remove_file(&transcript_path) {
            Ok(()) => info!(path = %transcript_path.display(), "deleted transcript"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(format!(
                    "could not delete {}: {e}",
                    transcript_path.display()
                ))
            }
        }

        // Delete every saved agent run by walking the directory and
        // removing each .json file. We use the per-agent delete path
        // (rather than rm -rf) so we keep the dir itself around with
        // any future siblings (audit log etc.) intact.
        let runs_dir = AgentRunStore::dir(&dir);
        if runs_dir.is_dir() {
            for entry in std::fs::read_dir(&runs_dir).map_err(|e| {
                format!(
                    "could not read agent_runs dir at {}: {e}",
                    runs_dir.display()
                )
            })? {
                let entry = entry.map_err(|e| format!("agent_runs entry read error: {e}"))?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Err(e) = std::fs::remove_file(&path) {
                        return Err(format!(
                            "could not delete agent run {}: {e}",
                            path.display()
                        ));
                    }
                }
            }
            // Remove the empty directory if it's empty now.
            let _ = std::fs::remove_dir(&runs_dir);
            info!(path = %runs_dir.display(), "cleared agent_runs");
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("clear_recording_artifacts task panicked: {e}"))?
}

/// Build a vault-snapshot zip at `destination`. The destination is
/// chosen by the user via the native save dialog on the frontend; we
/// accept it here as an absolute path. The snapshot bundles the
/// current settings, tasks file, recordings tree, and memory tree —
/// see `attune_core::storage::snapshot` for the on-disk layout.
///
/// v2 finding 057 / GET-92. Restore + scheduled drops are tracked as
/// follow-ups; this command lands the export half of the contract so
/// the user can already drag the resulting zip into iCloud / Dropbox.
#[tauri::command]
pub async fn export_vault_snapshot(
    destination: PathBuf,
    state: State<'_, AppState>,
) -> Result<SnapshotSummary, String> {
    // Capture every path off the AppState's settings + store before
    // the spawn_blocking jump so we don't hold the lock across the
    // worker boundary.
    let paths = {
        let settings = state.settings.lock();
        SnapshotPaths {
            recordings_dir: settings.output_dir.clone(),
            memory_dir: settings.memory_dir.clone(),
            tasks_path: settings.tasks_path.clone(),
            settings_path: state.settings_store.path().to_path_buf(),
        }
    };

    tauri::async_runtime::spawn_blocking(move || -> Result<SnapshotSummary, String> {
        let summary = export_snapshot_impl(&destination, &paths).map_err(|e| e.to_string())?;
        info!(
            destination = %summary.destination.display(),
            files = summary.files,
            bytes = summary.bytes,
            "vault snapshot exported"
        );
        Ok(summary)
    })
    .await
    .map_err(|e| format!("export_vault_snapshot task panicked: {e}"))?
}

/// Walk every session under `recordings_dir` and delete mic.wav +
/// system.wav from sessions where the source audio is at least
/// `older_than_days` old AND a transcript is already on disk. v2
/// finding 063 / GET-98. When `older_than_days` is None we read the
/// retention from the active settings; when both are missing the
/// command returns a zero-summary so the UI's 'Purge now' button
/// can fire safely on any configuration.
#[tauri::command]
pub async fn purge_old_wav_files(
    state: State<'_, AppState>,
    older_than_days: Option<u32>,
) -> Result<PurgeSummary, String> {
    let (recordings_dir, effective_days) = {
        let settings = state.settings.lock();
        let days = older_than_days.or(settings.wav_retention_days).unwrap_or(0);
        (settings.output_dir.clone(), days)
    };
    if effective_days == 0 {
        return Ok(PurgeSummary {
            sessions_inspected: 0,
            wavs_deleted: 0,
            bytes_freed: 0,
            failed: Vec::new(),
        });
    }
    tauri::async_runtime::spawn_blocking(move || -> Result<PurgeSummary, String> {
        let summary = purge_old_wavs(&recordings_dir, effective_days);
        info!(
            recordings = %recordings_dir.display(),
            older_than_days = effective_days,
            inspected = summary.sessions_inspected,
            deleted = summary.wavs_deleted,
            bytes = summary.bytes_freed,
            "wav retention sweep complete"
        );
        Ok(summary)
    })
    .await
    .map_err(|e| format!("purge_old_wav_files task panicked: {e}"))?
}
