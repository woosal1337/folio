use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{FolioError, Result};

pub const SPEAKERS_FILENAME: &str = "speakers.json";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionSpeaker {
    pub cluster: i32,

    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub registry_id: Option<String>,

    #[serde(default)]
    pub auto_named: bool,

    #[serde(default)]
    pub embedding: Vec<f32>,

    #[serde(default)]
    pub suggested_name: Option<String>,

    #[serde(default)]
    pub suggested_registry_id: Option<String>,

    #[serde(default)]
    pub suggested_score: Option<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct SpeakerLabel {
    pub cluster: i32,

    pub name: Option<String>,

    pub auto_named: bool,

    pub has_embedding: bool,

    pub suggested_name: Option<String>,

    pub suggested_score: Option<f32>,
}

impl SessionSpeaker {
    fn to_label(&self) -> SpeakerLabel {
        SpeakerLabel {
            cluster: self.cluster,
            name: self.name.clone(),
            auto_named: self.auto_named,
            has_embedding: !self.embedding.is_empty(),

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

    pub fn read(session_dir: &Path) -> Result<Option<Self>> {
        let path = Self::path_in(session_dir);
        match std::fs::read(&path) {
            Ok(bytes) => {
                let v: Self = serde_json::from_slice(&bytes)
                    .map_err(|e| FolioError::Storage(format!("{SPEAKERS_FILENAME} parse: {e}")))?;
                Ok(Some(v))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(FolioError::Storage(format!(
                "{SPEAKERS_FILENAME} read {}: {e}",
                path.display()
            ))),
        }
    }

    pub fn write(&self, session_dir: &Path) -> Result<()> {
        let path = Self::path_in(session_dir);
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| FolioError::Storage(format!("{SPEAKERS_FILENAME} serialize: {e}")))?;
        crate::storage::atomic_write::atomic_write(&path, &bytes)
    }

    pub fn get(&self, cluster: i32) -> Option<&SessionSpeaker> {
        self.speakers.iter().find(|s| s.cluster == cluster)
    }

    pub fn get_mut(&mut self, cluster: i32) -> Option<&mut SessionSpeaker> {
        self.speakers.iter_mut().find(|s| s.cluster == cluster)
    }

    pub fn labels(&self) -> Vec<SpeakerLabel> {
        self.speakers.iter().map(SessionSpeaker::to_label).collect()
    }

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

        let map = s.name_map();
        assert_eq!(map.get(&0).map(String::as_str), Some("Alice"));
        assert!(!map.contains_key(&1));
    }
}
