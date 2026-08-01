use std::path::PathBuf;

use folio_core::llm::AgentRunStore;
use folio_core::storage::digest::{
    default_digests_dir, generate as generate_digest_impl, DigestPaths, DigestResult,
};
use folio_core::storage::git_sync::{is_git_repo, sync as git_sync_impl, GitSyncSummary};
use folio_core::storage::retention::{purge_old_wavs, PurgeSummary};
use folio_core::storage::snapshot::{
    export as export_snapshot_impl, SnapshotPaths, SnapshotSummary,
};
use tauri::State;
use tracing::info;

use crate::app::AppState;

const TRANSCRIPT_FILENAME: &str = "transcript.json";

#[tauri::command]
pub async fn clear_recording_artifacts(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<(), String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let dir = folio_core::paths::canonicalize_under(&output_dir, &session_dir)
            .map_err(|e| e.to_string())?;
        for filename in [TRANSCRIPT_FILENAME, "transcript.json.zst"] {
            let transcript_path = dir.join(filename);
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
        }

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

            let _ = std::fs::remove_dir(&runs_dir);
            info!(path = %runs_dir.display(), "cleared agent_runs");
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("clear_recording_artifacts task panicked: {e}"))?
}

#[tauri::command]
pub async fn export_vault_snapshot(
    destination: PathBuf,
    state: State<'_, AppState>,
) -> Result<SnapshotSummary, String> {
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
        check_export_destination(&destination)?;
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

fn check_export_destination(destination: &std::path::Path) -> Result<(), String> {
    const DENYLIST: &[&str] = &[
        "/etc/",
        "/System/",
        "/Library/",
        "/usr/",
        "/private/etc/",
        "/private/var/",
        "/sbin/",
        "/bin/",
    ];
    let canonical = destination
        .parent()
        .and_then(|p| std::fs::canonicalize(p).ok())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| destination.to_string_lossy().to_string());
    let lc = canonical.to_lowercase();
    for prefix in DENYLIST {
        if lc.starts_with(&prefix.to_lowercase()) {
            return Err(format!(
                "refused export to {} — destination is under a protected system directory",
                canonical
            ));
        }
    }
    Ok(())
}

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

#[tauri::command]
pub async fn generate_weekly_digest(state: State<'_, AppState>) -> Result<DigestResult, String> {
    let paths = {
        let settings = state.settings.lock();
        DigestPaths {
            recordings_dir: settings.output_dir.clone(),
            memory_dir: settings.memory_dir.clone(),
            tasks_path: settings.tasks_path.clone(),
            digests_dir: default_digests_dir(),
        }
    };
    tauri::async_runtime::spawn_blocking(move || -> Result<DigestResult, String> {
        let result = generate_digest_impl(&paths).map_err(|e| e.to_string())?;
        info!(
            path = %result.path.display(),
            recordings = result.recordings,
            aged_tasks = result.aged_tasks,
            new_memories = result.new_memories,
            "weekly digest generated"
        );
        Ok(result)
    })
    .await
    .map_err(|e| format!("generate_weekly_digest task panicked: {e}"))?
}

#[tauri::command]
pub async fn git_sync_vault(state: State<'_, AppState>) -> Result<GitSyncSummary, String> {
    let vault_dir = {
        let settings = state.settings.lock();
        settings.memory_dir.clone()
    };
    tauri::async_runtime::spawn_blocking(move || -> Result<GitSyncSummary, String> {
        let summary = git_sync_impl(&vault_dir);
        info!(
            dir = %vault_dir.display(),
            is_repo = summary.is_repo,
            committed = summary.committed,
            ok = summary.ok,
            "git sync attempt"
        );
        Ok(summary)
    })
    .await
    .map_err(|e| format!("git_sync_vault task panicked: {e}"))?
}

#[tauri::command]
pub async fn git_vault_is_repo(state: State<'_, AppState>) -> Result<bool, String> {
    let vault_dir = {
        let settings = state.settings.lock();
        settings.memory_dir.clone()
    };
    tauri::async_runtime::spawn_blocking(move || -> Result<bool, String> {
        Ok(is_git_repo(&vault_dir))
    })
    .await
    .map_err(|e| format!("git_vault_is_repo task panicked: {e}"))?
}
