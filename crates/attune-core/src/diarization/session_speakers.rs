//! Per-recording speaker sidecar — `<session>/speakers.json` (GET-189).
//!
//! After diarization, each system-channel cluster (`Speaker 1/2/3…`) gets
//! a [`SessionSpeaker`]: a representative voice embedding plus, once known,
//! a display name and the registry identity it links to. This is what
//! makes a rename stick to *this* recording and — via the embedding —
//! teachable to the cross-recording [`crate::speaker_memory`] registry.
//!
//! The embedding is biometric, so it never crosses the IPC boundary: the
//! frontend gets the lightweight [`SpeakerLabel`] (cluster + name +
//! provenance) instead.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AttuneError, Result};

/// Sidecar filename inside a session directory.
pub const SPEAKERS_FILENAME: &str = "speakers.json";

/// One diarized cluster's identity within a single recording.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionSpeaker {
    /// Diarizer cluster id — matches `TranscriptSegment.speaker`.
    pub cluster: i32,
    /// Resolved display name, when known (auto-matched or user-set).
    #[serde(default)]
    pub name: Option<String>,
    /// Linked registry identity (UUID string), when matched or named.
    #[serde(default)]
    pub registry_id: Option<String>,
    /// True when `name` came from an automatic registry match rather than a
    /// user rename. A user rename always wins over an auto match.
    #[serde(default)]
    pub auto_named: bool,
    /// Representative embedding for this cluster (256-d, L2-normalizable).
    /// Empty when the cluster had too little audio to embed. Kept so a
    /// later rename can teach the registry without re-running the audio.
    #[serde(default)]
    pub embedding: Vec<f32>,
    /// A medium-confidence registry match (GET-189 Confirm tier): the
    /// candidate name to prompt "Is this <name>?" on. Set only while `name`
    /// is unset; cleared on confirm / reject / rename.
    #[serde(default)]
    pub suggested_name: Option<String>,
    /// Registry identity (UUID string) the suggestion points at.
    #[serde(default)]
    pub suggested_registry_id: Option<String>,
    /// Match confidence (cosine, 0–1) for the suggestion, for display.
    #[serde(default)]
    pub suggested_score: Option<f32>,
}

/// Lightweight, embedding-free view of a session speaker for the frontend.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct SpeakerLabel {
    /// Diarizer cluster id — matches `TranscriptSegment.speaker`.
    pub cluster: i32,
    /// User-facing name, when set; `null` falls back to "Speaker N" in the UI.
    pub name: Option<String>,
    /// True when the name was filled by an automatic registry match.
    pub auto_named: bool,
    /// True when this cluster carries a usable voice embedding (i.e. it can
    /// be remembered). False for clusters with too little audio.
    pub has_embedding: bool,
    /// Candidate name for a medium-confidence match to confirm ("Is this
    /// <name>?"). `null` when there's no pending suggestion.
    pub suggested_name: Option<String>,
    /// Match confidence (cosine, 0–1) for the suggestion, for display.
    pub suggested_score: Option<f32>,
}

impl SessionSpeaker {
    fn to_label(&self) -> SpeakerLabel {
        SpeakerLabel {
            cluster: self.cluster,
            name: self.name.clone(),
            auto_named: self.auto_named,
            has_embedding: !self.embedding.is_empty(),
            // Only surface a suggestion while the speaker is still unnamed.
            suggested_name: if self.name.is_none() {
                self.suggested_name.clone()
            } else {
                None
            },
            suggested_score: if self.name.is_none() {
                self.suggested_score
            } else {
                None
            },
        }
    }
}

/// The whole sidecar.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SessionSpeakers {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub speakers: Vec<SessionSpeaker>,
}

impl SessionSpeakers {
    pub fn path_in(session_dir: &Path) -> PathBuf {
        session_dir.join(SPEAKERS_FILENAME)
    }

    /// Read the sidecar. A missing file yields `None` (the recording was
    /// never diarized, or predates this feature).
    pub fn read(session_dir: &Path) -> Result<Option<Self>> {
        let path = Self::path_in(session_dir);
        match std::fs::read(&path) {
            Ok(bytes) => {
                let v: Self = serde_json::from_slice(&bytes).map_err(|e| {
                    AttuneError::Storage(format!("{SPEAKERS_FILENAME} parse: {e}"))
                })?;
                Ok(Some(v))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AttuneError::Storage(format!(
                "{SPEAKERS_FILENAME} read {}: {e}",
                path.display()
            ))),
        }
    }

    /// Atomically write the sidecar into the session directory.
    pub fn write(&self, session_dir: &Path) -> Result<()> {
        let path = Self::path_in(session_dir);
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| AttuneError::Storage(format!("{SPEAKERS_FILENAME} serialize: {e}")))?;
        crate::storage::atomic_write::atomic_write(&path, &bytes)
    }

    pub fn get(&self, cluster: i32) -> Option<&SessionSpeaker> {
        self.speakers.iter().find(|s| s.cluster == cluster)
    }

    pub fn get_mut(&mut self, cluster: i32) -> Option<&mut SessionSpeaker> {
        self.speakers.iter_mut().find(|s| s.cluster == cluster)
    }

    /// Embedding-free view for the IPC layer.
    pub fn labels(&self) -> Vec<SpeakerLabel> {
        self.speakers.iter().map(SessionSpeaker::to_label).collect()
    }

    /// Map of cluster id → resolved display name, for the transcript
    /// formatters. Only clusters with a name are included.
    pub fn name_map(&self) -> HashMap<i32, String> {
        self.speakers
            .iter()
            .filter_map(|s| s.name.as_ref().map(|n| (s.cluster, n.clone())))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn speaker(cluster: i32, name: Option<&str>, embedding: Vec<f32>) -> SessionSpeaker {
        SessionSpeaker {
            cluster,
            name: name.map(str::to_string),
            registry_id: None,
            auto_named: false,
            embedding,
            suggested_name: None,
            suggested_registry_id: None,
            suggested_score: None,
        }
    }

    #[test]
    fn read_missing_is_none() {
        let dir = TempDir::new().unwrap();
        assert!(SessionSpeakers::read(dir.path()).unwrap().is_none());
    }

    #[test]
    fn write_then_read_round_trips() {
        let dir = TempDir::new().unwrap();
        let s = SessionSpeakers {
            version: 1,
            speakers: vec![
                speaker(0, Some("Alice"), vec![0.1, 0.2]),
                speaker(2, None, vec![]),
            ],
        };
        s.write(dir.path()).unwrap();
        let back = SessionSpeakers::read(dir.path()).unwrap().unwrap();
        assert_eq!(back.speakers, s.speakers);
        assert_eq!(back.get(0).unwrap().name.as_deref(), Some("Alice"));
    }

    #[test]
    fn labels_hide_embeddings_and_name_map_filters() {
        let s = SessionSpeakers {
            version: 1,
            speakers: vec![
                speaker(0, Some("Alice"), vec![0.1, 0.2, 0.3]),
                speaker(1, None, vec![]),
            ],
        };
        let labels = s.labels();
        assert_eq!(labels.len(), 2);
        assert!(labels[0].has_embedding);
        assert!(!labels[1].has_embedding);
        // Embeddings never appear in the wire DTO (it has no such field).
        let map = s.name_map();
        assert_eq!(map.get(&0).map(String::as_str), Some("Alice"));
        assert!(!map.contains_key(&1));
    }
}
