//! One-off maintenance commands the frontend calls when the user wants
//! to undo work and start over on a recording. Today: clearing
//! transcripts so they can be regenerated against the latest pipeline.

use std::path::PathBuf;

use attune_core::llm::AgentRunStore;
use tracing::info;

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
