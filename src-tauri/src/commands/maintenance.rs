use std::path::PathBuf;

use folio_core::llm::AgentRunStore;
use folio_core::storage::digest::{
    default_digests_dir, generate as generate_digest_impl, DigestPaths, DigestResult,
};
use folio_core::storage::fs_io::{
    archive_inbox_entry as archive_inbox_impl, list_inbox as list_inbox_impl, InboxEntry,
};
use folio_core::storage::git_sync::{is_git_repo, sync as git_sync_impl, GitSyncSummary};
use folio_core::storage::retention::{purge_old_wavs, PurgeSummary};
use folio_core::storage::share_bundle::{export as export_share_bundle_impl, ShareBundleSummary};
use folio_core::storage::showcase::{read as read_showcase, write as write_showcase, Showcase};
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
pub async fn export_share_bundle(
    state: State<'_, AppState>,
    session_dir: PathBuf,
    destination: PathBuf,
) -> Result<ShareBundleSummary, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<ShareBundleSummary, String> {
        let session_dir = folio_core::paths::canonicalize_under(&output_dir, &session_dir)
            .map_err(|e| e.to_string())?;
        check_export_destination(&destination)?;
        let summary =
            export_share_bundle_impl(&session_dir, &destination).map_err(|e| e.to_string())?;
        info!(
            destination = %summary.destination.display(),
            files = summary.files,
            bytes = summary.bytes,
            manifest = %summary.manifest_sha256,
            "share bundle exported"
        );
        Ok(summary)
    })
    .await
    .map_err(|e| format!("export_share_bundle task panicked: {e}"))?
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

#[tauri::command]
pub async fn list_inbox_entries(state: State<'_, AppState>) -> Result<Vec<InboxEntry>, String> {
    let memory_dir = {
        let settings = state.settings.lock();
        settings.memory_dir.clone()
    };
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<InboxEntry>, String> {
        Ok(list_inbox_impl(&memory_dir))
    })
    .await
    .map_err(|e| format!("list_inbox_entries task panicked: {e}"))?
}

#[tauri::command]
pub async fn archive_inbox_entry(state: State<'_, AppState>, path: PathBuf) -> Result<(), String> {
    let memory_dir = state.settings.lock().memory_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let canon =
            folio_core::paths::canonicalize_under(&memory_dir, &path).map_err(|e| e.to_string())?;
        archive_inbox_impl(&canon).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| format!("archive_inbox_entry task panicked: {e}"))?
}

#[tauri::command]
pub async fn get_showcase(state: State<'_, AppState>) -> Result<Option<Showcase>, String> {
    let memory_dir = {
        let settings = state.settings.lock();
        settings.memory_dir.clone()
    };
    tauri::async_runtime::spawn_blocking(move || -> Result<Option<Showcase>, String> {
        read_showcase(&memory_dir).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("get_showcase task panicked: {e}"))?
}

#[tauri::command]
pub async fn save_showcase(state: State<'_, AppState>, showcase: Showcase) -> Result<(), String> {
    let memory_dir = {
        let settings = state.settings.lock();
        settings.memory_dir.clone()
    };
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        write_showcase(&memory_dir, &showcase).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| format!("save_showcase task panicked: {e}"))?
}

#[tauri::command]
pub async fn apply_cross_track_aec(
    state: State<'_, AppState>,
    session_dir: String,
) -> Result<String, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    let session_path =
        folio_core::paths::canonicalize_under(&output_dir, std::path::Path::new(&session_dir))
            .map_err(|e| e.to_string())?;

    tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let mic_path = session_path.join("mic.wav");
        let sys_path = session_path.join("system.wav");
        let out_path = session_path.join("mic.aec.wav");

        if !mic_path.exists() || !sys_path.exists() {
            return Err("session must have both mic.wav and system.wav for cross-track AEC".into());
        }

        folio_core::audio::enhancement::cross_track_aec::apply_aec(&mic_path, &sys_path, &out_path)
            .map_err(|e| e.to_string())?;

        Ok(out_path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| format!("cross_track_aec task panicked: {e}"))?
}
