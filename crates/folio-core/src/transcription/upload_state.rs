use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{FolioError, Result};

const STATE_FILE: &str = "upload-state.json";

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ChunkStatus {
    Pending,
    Uploading { attempts: u32 },
    Succeeded { transcript_text: String },
    Failed { attempts: u32, last_error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkRecord {
    pub index: usize,
    pub start_sample: usize,
    pub end_sample: usize,
    pub bytes: usize,
    pub status: ChunkStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UploadState {
    pub version: u32,
    pub started_at: DateTime<Utc>,
    pub chunks: Vec<ChunkRecord>,
}

impl UploadState {
    pub fn new(chunks: Vec<ChunkRecord>) -> Self {
        Self {
            version: 1,
            started_at: Utc::now(),
            chunks,
        }
    }

    pub fn remaining(&self) -> Vec<&ChunkRecord> {
        self.chunks
            .iter()
            .filter(|c| !matches!(c.status, ChunkStatus::Succeeded { .. }))
            .collect()
    }

    pub fn is_complete(&self) -> bool {
        self.chunks
            .iter()
            .all(|c| matches!(c.status, ChunkStatus::Succeeded { .. }))
    }

    pub fn stitched_transcript(&self) -> Option<String> {
        let mut out = String::new();
        for chunk in &self.chunks {
            match &chunk.status {
                ChunkStatus::Succeeded { transcript_text } => {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(transcript_text.trim());
                }
                _ => return None,
            }
        }
        Some(out)
    }

    pub fn mark_succeeded(&mut self, index: usize, transcript_text: String) {
        if let Some(chunk) = self.chunks.get_mut(index) {
            chunk.status = ChunkStatus::Succeeded { transcript_text };
        }
    }

    pub fn mark_failed(&mut self, index: usize, error: String) {
        if let Some(chunk) = self.chunks.get_mut(index) {
            let attempts = match &chunk.status {
                ChunkStatus::Failed { attempts, .. } => attempts + 1,
                ChunkStatus::Uploading { attempts } => attempts + 1,
                _ => 1,
            };
            chunk.status = ChunkStatus::Failed {
                attempts,
                last_error: error,
            };
        }
    }
}

pub fn state_path(session_dir: &Path) -> std::path::PathBuf {
    session_dir.join(STATE_FILE)
}

pub fn load(session_dir: &Path) -> Result<Option<UploadState>> {
    let path = state_path(session_dir);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read(&path)
        .map_err(|e| FolioError::Storage(format!("could not read {}: {e}", path.display())))?;
    let state = serde_json::from_slice::<UploadState>(&raw).map_err(|e| {
        FolioError::Storage(format!("invalid upload-state JSON {}: {e}", path.display()))
    })?;
    Ok(Some(state))
}

pub fn save(session_dir: &Path, state: &UploadState) -> Result<()> {
    let final_path = state_path(session_dir);
    let tmp_path = final_path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| FolioError::Storage(format!("could not serialise upload state: {e}")))?;
    fs::write(&tmp_path, json)
        .map_err(|e| FolioError::Storage(format!("could not write {}: {e}", tmp_path.display())))?;
    fs::rename(&tmp_path, &final_path).map_err(|e| {
        FolioError::Storage(format!("could not rename {}: {e}", final_path.display()))
    })?;
    Ok(())
}

pub fn backoff_delay_secs(attempts: u32) -> f64 {
    let base = 2.0_f64.powi(attempts.min(6) as i32);
    let jitter = ((attempts as f64 * 0.37).fract() - 0.5) * 0.4 * base;
    (base + jitter).clamp(0.1, 60.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(index: usize, status: ChunkStatus) -> ChunkRecord {
        ChunkRecord {
            index,
            start_sample: index * 1000,
            end_sample: (index + 1) * 1000,
            bytes: 2000,
            status,
        }
    }

    #[test]
    fn remaining_filters_succeeded_chunks() {
        let state = UploadState::new(vec![
            chunk(
                0,
                ChunkStatus::Succeeded {
                    transcript_text: "hi".into(),
                },
            ),
            chunk(1, ChunkStatus::Pending),
            chunk(
                2,
                ChunkStatus::Failed {
                    attempts: 2,
                    last_error: "timeout".into(),
                },
            ),
        ]);
        let remaining: Vec<usize> = state.remaining().iter().map(|c| c.index).collect();
        assert_eq!(remaining, vec![1, 2]);
    }

    #[test]
    fn is_complete_only_when_every_chunk_succeeded() {
        let mut state = UploadState::new(vec![
            chunk(
                0,
                ChunkStatus::Succeeded {
                    transcript_text: "hi".into(),
                },
            ),
            chunk(1, ChunkStatus::Pending),
        ]);
        assert!(!state.is_complete());
        state.mark_succeeded(1, "bye".into());
        assert!(state.is_complete());
    }

    #[test]
    fn stitched_transcript_joins_with_single_space() {
        let mut state = UploadState::new(vec![
            chunk(0, ChunkStatus::Pending),
            chunk(1, ChunkStatus::Pending),
        ]);
        state.mark_succeeded(0, "hello world".into());
        state.mark_succeeded(1, "second chunk".into());
        assert_eq!(
            state.stitched_transcript().as_deref(),
            Some("hello world second chunk")
        );
    }

    #[test]
    fn stitched_transcript_is_none_when_anything_unfinished() {
        let state = UploadState::new(vec![chunk(0, ChunkStatus::Pending)]);
        assert!(state.stitched_transcript().is_none());
    }

    #[test]
    fn mark_failed_bumps_attempts() {
        let mut state = UploadState::new(vec![chunk(0, ChunkStatus::Pending)]);
        state.mark_failed(0, "timeout".into());
        state.mark_failed(0, "timeout".into());
        match &state.chunks[0].status {
            ChunkStatus::Failed { attempts, .. } => assert_eq!(*attempts, 2),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let state = UploadState::new(vec![chunk(0, ChunkStatus::Pending)]);
        save(dir.path(), &state).unwrap();
        let loaded = load(dir.path()).unwrap().unwrap();
        assert_eq!(loaded, state);
    }

    #[test]
    fn load_returns_none_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(dir.path()).unwrap().is_none());
    }

    #[test]
    fn backoff_delay_grows_and_caps_at_60() {
        let d1 = backoff_delay_secs(1);
        let d3 = backoff_delay_secs(3);
        let d10 = backoff_delay_secs(10);
        assert!(d3 > d1);
        assert!(d10 <= 60.0);
    }
}
