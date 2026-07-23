use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{FolioError, Result};
use crate::storage::atomic_write::atomic_write_json;

const SYNC_FILE: &str = "sync.json";
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../src/shared/types/")]
pub enum UploadPhase {
    Pending,
    Uploading,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../src/shared/types/")]
pub enum RemoteStatus {
    None,
    Queued,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct SyncState {
    pub schema_version: u32,
    pub recording_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_recording_id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_job_id: Option<String>,

    pub upload_state: UploadPhase,
    pub remote_status: RemoteStatus,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<DateTime<Utc>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl SyncState {
    pub fn new(recording_id: String) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            recording_id,
            remote_recording_id: None,
            remote_job_id: None,
            upload_state: UploadPhase::Pending,
            remote_status: RemoteStatus::None,
            last_synced_at: None,
            error: None,
        }
    }
}

pub fn path(session_dir: &Path) -> PathBuf {
    session_dir.join(SYNC_FILE)
}

pub fn load(session_dir: &Path) -> Result<Option<SyncState>> {
    let path = path(session_dir);
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read(&path)
        .map_err(|e| FolioError::Storage(format!("read {}: {e}", path.display())))?;
    let state = serde_json::from_slice::<SyncState>(&raw)
        .map_err(|e| FolioError::Storage(format!("invalid sync.json {}: {e}", path.display())))?;
    Ok(Some(state))
}

pub fn save(session_dir: &Path, state: &SyncState) -> Result<()> {
    atomic_write_json(&path(session_dir), state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(dir.path()).unwrap().is_none());

        let mut state = SyncState::new("client-uuid-1".into());
        state.remote_recording_id = Some("srv-1".into());
        state.upload_state = UploadPhase::Complete;
        state.remote_status = RemoteStatus::Queued;
        save(dir.path(), &state).unwrap();

        let loaded = load(dir.path()).unwrap().unwrap();
        assert_eq!(loaded, state);
        assert_eq!(loaded.recording_id, "client-uuid-1");
    }
}
