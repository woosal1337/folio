//! One-off maintenance commands the frontend calls when the user wants
//! to undo work and start over on a recording. Today: clearing
//! transcripts so they can be regenerated against the latest pipeline,
//! plus building a vault-snapshot zip (v2 finding 057 / GET-92).

use std::path::PathBuf;

use attune_core::llm::AgentRunStore;
use attune_core::storage::digest::{
    default_digests_dir, generate as generate_digest_impl, DigestPaths, DigestResult,
};
use attune_core::storage::fs_io::{
    archive_inbox_entry as archive_inbox_impl, list_inbox as list_inbox_impl, InboxEntry,
};
use attune_core::storage::git_sync::{is_git_repo, sync as git_sync_impl, GitSyncSummary};
use attune_core::storage::retention::{purge_old_wavs, PurgeSummary};
use attune_core::storage::share_bundle::{export as export_share_bundle_impl, ShareBundleSummary};
use attune_core::storage::showcase::{read as read_showcase, write as write_showcase, Showcase};
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
pub async fn clear_recording_artifacts(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<(), String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let dir = attune_core::paths::canonicalize_under(&output_dir, &session_dir)
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

/// Defence-in-depth deny-list for export destinations. The native save
/// dialog already restricts users to writable directories, but a
/// frontend bug or a deep-link could still hand us a path aimed at a
/// system directory. Refuse anything that lands inside one of these
/// roots. v2 finding R02 / Phase-3 audit B8.
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

/// Generate a weekly digest markdown file and return where it landed.
/// v2 finding 082 / GET-80. Reads the current settings to resolve
/// recordings/memory/tasks paths; the digest itself goes to
/// `~/Documents/Attune/Digests/` by default.
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

/// Export a single recording as a sealed .attune-share zip with a
/// manifest carrying SHA-256 hashes of every file inside. v2 finding
/// 052 / GET-69.
#[tauri::command]
pub async fn export_share_bundle(
    state: State<'_, AppState>,
    session_dir: PathBuf,
    destination: PathBuf,
) -> Result<ShareBundleSummary, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<ShareBundleSummary, String> {
        let session_dir = attune_core::paths::canonicalize_under(&output_dir, &session_dir)
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

/// Sync the user's vault (memory dir) via the system git binary.
/// v2 finding 070 / GET-72. Pulls with rebase + autostash, commits
/// any local changes with a generic 'attune sync' message, then
/// pushes. Returns the structured summary so the UI can render the
/// outcome without parsing git output.
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

/// Cheap check: is the vault dir under version control? UI uses
/// this to decide whether to surface the Sync card.
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

/// List pending inbox entries from `<memory_dir>/.attune/inbox/`.
/// v2 finding 073 / GET-75.
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

/// Archive a single inbox entry (rename into .processed/).
/// v2 finding 073 / GET-75.
#[tauri::command]
pub async fn archive_inbox_entry(state: State<'_, AppState>, path: PathBuf) -> Result<(), String> {
    let memory_dir = state.settings.lock().memory_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let canon = attune_core::paths::canonicalize_under(&memory_dir, &path)
            .map_err(|e| e.to_string())?;
        archive_inbox_impl(&canon).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| format!("archive_inbox_entry task panicked: {e}"))?
}

/// Load the user's showcase if one exists.
/// v2 finding 087 / GET-107.
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

/// Persist the user's showcase. v2 finding 087 / GET-107.
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

/// Apply cross-track AEC to a session's mic.wav using system.wav as the
/// reference signal (GET-202). Writes `mic.aec.wav` next to the originals.
/// Returns the output path as a string.
#[tauri::command]
pub async fn apply_cross_track_aec(
    state: State<'_, AppState>,
    session_dir: String,
) -> Result<String, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    let session_path =
        attune_core::paths::canonicalize_under(&output_dir, std::path::Path::new(&session_dir))
            .map_err(|e| e.to_string())?;

    tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let mic_path = session_path.join("mic.wav");
        let sys_path = session_path.join("system.wav");
        let out_path = session_path.join("mic.aec.wav");

        if !mic_path.exists() || !sys_path.exists() {
            return Err("session must have both mic.wav and system.wav for cross-track AEC".into());
        }

        attune_core::audio::enhancement::cross_track_aec::apply_aec(
            &mic_path, &sys_path, &out_path,
        )
        .map_err(|e| e.to_string())?;

        Ok(out_path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| format!("cross_track_aec task panicked: {e}"))?
}
