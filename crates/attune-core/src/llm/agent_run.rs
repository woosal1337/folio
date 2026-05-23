//! On-disk persistence for agent runs against a recording.
//!
//! Phase 1.5 MVP design: one JSON file per (recording, agent), at
//! `<recording_session_dir>/agent_runs/<agent_id>.json`. Re-running an
//! agent overwrites its file — we only keep the latest result for the
//! MVP. Multi-history persistence + the vault-rooted location described
//! in the plan land in phase 3.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AttuneError, Result};
use crate::llm::ProviderId;

/// Subdirectory inside each recording where agent results live. Hidden
/// from the user's `ls` by convention but easy to spot in Finder.
const AGENT_RUNS_DIR: &str = "agent_runs";

/// One agent's most recent run on a recording. Persisted as JSON.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct AgentRun {
    pub agent_id: String,
    pub agent_name: String,
    pub provider: ProviderId,
    pub model: String,
    /// The model's response text. Markdown is allowed and expected.
    pub response: String,
    /// Set if the provider returned token usage.
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    /// ISO-8601 timestamp the run finished at.
    pub finished_at: DateTime<Utc>,
}

/// Read/write [`AgentRun`] artifacts under a recording's session dir.
pub struct AgentRunStore;

impl AgentRunStore {
    /// Path to the agent-runs subdirectory inside a recording.
    pub fn dir(session_dir: &Path) -> PathBuf {
        session_dir.join(AGENT_RUNS_DIR)
    }

    /// Path to a specific agent's run file.
    pub fn path(session_dir: &Path, agent_id: &str) -> PathBuf {
        Self::dir(session_dir).join(format!("{}.json", agent_id))
    }

    /// Persist a run. Creates the agent_runs/ directory if missing.
    /// Atomic-ish (write to temp + rename) so a crash mid-write cannot
    /// leave a half-written JSON file.
    pub fn save(session_dir: &Path, run: &AgentRun) -> Result<PathBuf> {
        let dir = Self::dir(session_dir);
        std::fs::create_dir_all(&dir).map_err(|e| {
            AttuneError::Storage(format!(
                "could not create agent_runs dir at {}: {e}",
                dir.display()
            ))
        })?;

        let final_path = Self::path(session_dir, &run.agent_id);
        let tmp_path = final_path.with_extension("json.tmp");

        let json = serde_json::to_string_pretty(run)
            .map_err(|e| AttuneError::Storage(format!("could not serialise agent run: {e}")))?;
        std::fs::write(&tmp_path, json).map_err(|e| {
            AttuneError::Storage(format!(
                "could not write agent run temp {}: {e}",
                tmp_path.display()
            ))
        })?;
        std::fs::rename(&tmp_path, &final_path).map_err(|e| {
            AttuneError::Storage(format!("could not rename agent run temp into place: {e}"))
        })?;
        Ok(final_path)
    }

    /// Load all agent runs for a recording. Returns an empty vec if the
    /// agent_runs/ directory does not exist. Files that fail to parse
    /// are skipped (with a tracing warning); we never fail the whole
    /// list because of one corrupt file.
    pub fn list(session_dir: &Path) -> Result<Vec<AgentRun>> {
        let dir = Self::dir(session_dir);
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(|e| {
            AttuneError::Storage(format!(
                "could not read agent_runs dir at {}: {e}",
                dir.display()
            ))
        })? {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(error = %e, "skipping unreadable agent_runs entry");
                    continue;
                }
            };
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            match std::fs::read_to_string(&path) {
                Ok(raw) => match serde_json::from_str::<AgentRun>(&raw) {
                    Ok(run) => out.push(run),
                    Err(e) => tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "skipping agent_runs entry that failed to parse",
                    ),
                },
                Err(e) => tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "skipping agent_runs entry that failed to read",
                ),
            }
        }
        out.sort_by(|a, b| a.agent_id.cmp(&b.agent_id));
        Ok(out)
    }

    /// Delete the saved run for a single agent on a recording.
    /// Idempotent — succeeds whether the file existed or not.
    pub fn delete(session_dir: &Path, agent_id: &str) -> Result<()> {
        let path = Self::path(session_dir, agent_id);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(AttuneError::Storage(format!(
                "could not delete agent run {}: {e}",
                path.display()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_run() -> AgentRun {
        AgentRun {
            agent_id: "summarize".to_string(),
            agent_name: "Summarize".to_string(),
            provider: ProviderId::OpenAi,
            model: "gpt-4o-mini".to_string(),
            response: "This meeting was about X.".to_string(),
            prompt_tokens: Some(1234),
            completion_tokens: Some(78),
            finished_at: Utc::now(),
        }
    }

    #[test]
    fn empty_dir_lists_empty() {
        let dir = TempDir::new().unwrap();
        let runs = AgentRunStore::list(dir.path()).unwrap();
        assert!(runs.is_empty());
    }

    #[test]
    fn save_then_list_round_trips() {
        let dir = TempDir::new().unwrap();
        let run = sample_run();
        AgentRunStore::save(dir.path(), &run).unwrap();
        let runs = AgentRunStore::list(dir.path()).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].agent_id, "summarize");
        assert_eq!(runs[0].response, "This meeting was about X.");
    }

    #[test]
    fn rerun_overwrites_previous() {
        let dir = TempDir::new().unwrap();
        let mut run = sample_run();
        AgentRunStore::save(dir.path(), &run).unwrap();
        run.response = "Updated summary.".to_string();
        AgentRunStore::save(dir.path(), &run).unwrap();
        let runs = AgentRunStore::list(dir.path()).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].response, "Updated summary.");
    }

    #[test]
    fn delete_is_idempotent() {
        let dir = TempDir::new().unwrap();
        AgentRunStore::delete(dir.path(), "summarize").unwrap(); // no-op
        AgentRunStore::save(dir.path(), &sample_run()).unwrap();
        AgentRunStore::delete(dir.path(), "summarize").unwrap();
        AgentRunStore::delete(dir.path(), "summarize").unwrap(); // no-op again
        assert!(AgentRunStore::list(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn list_skips_unparseable_files() {
        let dir = TempDir::new().unwrap();
        let runs_dir = AgentRunStore::dir(dir.path());
        std::fs::create_dir_all(&runs_dir).unwrap();
        std::fs::write(runs_dir.join("garbage.json"), "this is not json").unwrap();
        AgentRunStore::save(dir.path(), &sample_run()).unwrap();
        let runs = AgentRunStore::list(dir.path()).unwrap();
        // The good one survives, the garbage one is dropped.
        assert_eq!(runs.len(), 1);
    }
}
