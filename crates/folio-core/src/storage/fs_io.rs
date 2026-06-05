use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{FolioError, Result};

const OUTBOX_DIRNAME: &str = "outbox";
const INBOX_DIRNAME: &str = "inbox";

pub fn ensure_dirs(memory_dir: &Path) -> Result<PathBuf> {
    let root = memory_dir.join(".folio");
    for sub in ["", OUTBOX_DIRNAME, INBOX_DIRNAME] {
        let path = if sub.is_empty() {
            root.clone()
        } else {
            root.join(sub)
        };
        fs::create_dir_all(&path).map_err(|e| {
            FolioError::Storage(format!("could not create {}: {e}", path.display()))
        })?;
    }
    Ok(root)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub id: String,
    pub topic: String,
    pub emitted_at: String,
    pub data: serde_json::Value,
}

pub fn write_outbox_event(
    memory_dir: &Path,
    topic: &str,
    data: serde_json::Value,
) -> Result<PathBuf> {
    let root = ensure_dirs(memory_dir)?;
    let now = Utc::now();
    let id = Uuid::now_v7().to_string();
    let short = id.split('-').next().unwrap_or(&id).to_string();
    let safe_topic = topic.replace(['/', '\\', ' '], "-");
    let filename = format!(
        "{}_{}_{}.json",
        now.format("%Y%m%dT%H%M%SZ"),
        safe_topic,
        short
    );
    let path = root.join(OUTBOX_DIRNAME).join(filename);
    let entry = OutboxEntry {
        id,
        topic: topic.to_string(),
        emitted_at: now.to_rfc3339(),
        data,
    };
    let body = serde_json::to_vec_pretty(&entry)
        .map_err(|e| FolioError::Storage(format!("outbox serialize: {e}")))?;
    fs::write(&path, body)
        .map_err(|e| FolioError::Storage(format!("outbox write {}: {e}", path.display())))?;
    Ok(path)
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct InboxEntry {
    pub path: PathBuf,

    pub name: String,

    pub bytes: u64,

    pub modified_at: String,
}

pub fn list_inbox(memory_dir: &Path) -> Vec<InboxEntry> {
    let inbox = memory_dir.join(".folio").join(INBOX_DIRNAME);
    let entries = match fs::read_dir(&inbox) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<InboxEntry> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let modified_at = meta
            .modified()
            .ok()
            .map(chrono::DateTime::<Utc>::from)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();
        let name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_default();
        out.push(InboxEntry {
            path,
            name,
            bytes: meta.len(),
            modified_at,
        });
    }
    out.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    out
}

pub fn archive_inbox_entry(path: &Path) -> Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| FolioError::Storage("inbox entry has no parent".to_string()))?;
    let processed = parent.join(".processed");
    fs::create_dir_all(&processed).map_err(|e| {
        FolioError::Storage(format!(
            "could not create processed dir {}: {e}",
            processed.display()
        ))
    })?;
    let target = processed.join(
        path.file_name()
            .ok_or_else(|| FolioError::Storage("inbox entry has no filename".into()))?,
    );
    fs::rename(path, &target).map_err(|e| {
        FolioError::Storage(format!(
            "could not archive inbox entry {}: {e}",
            path.display()
        ))
    })?;
    Ok(target)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum InboxAction {
    #[serde(rename = "start-recording")]
    StartRecording,
    #[serde(rename = "stop-recording")]
    StopRecording,
    #[serde(rename = "record-this")]
    RecordThis { path: PathBuf },
    #[serde(other)]
    Other,
}

pub fn parse_inbox_action(raw: &str) -> Option<InboxAction> {
    serde_json::from_str(raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_dirs_creates_outbox_and_inbox() {
        let dir = tempfile::tempdir().unwrap();
        let root = ensure_dirs(dir.path()).unwrap();
        assert!(root.join(OUTBOX_DIRNAME).is_dir());
        assert!(root.join(INBOX_DIRNAME).is_dir());
    }

    #[test]
    fn write_outbox_event_lands_under_outbox() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_outbox_event(
            dir.path(),
            "recording.finished",
            serde_json::json!({"label": "smoke"}),
        )
        .unwrap();
        assert!(path.starts_with(dir.path().join(".folio").join(OUTBOX_DIRNAME)));
        let raw = std::fs::read_to_string(&path).unwrap();
        let parsed: OutboxEntry = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed.topic, "recording.finished");
        assert_eq!(parsed.data["label"], "smoke");
    }

    #[test]
    fn list_inbox_skips_non_json() {
        let dir = tempfile::tempdir().unwrap();
        let root = ensure_dirs(dir.path()).unwrap();
        std::fs::write(root.join(INBOX_DIRNAME).join("a.json"), "{}").unwrap();
        std::fs::write(root.join(INBOX_DIRNAME).join("ignored.txt"), "hi").unwrap();
        let listed = list_inbox(dir.path());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "a");
    }

    #[test]
    fn parse_inbox_action_understands_known_kinds() {
        let raw = r#"{"kind":"start-recording"}"#;
        let parsed = parse_inbox_action(raw).unwrap();
        assert!(matches!(parsed, InboxAction::StartRecording));

        let raw = r#"{"kind":"record-this","path":"/tmp/clip.wav"}"#;
        let parsed = parse_inbox_action(raw).unwrap();
        match parsed {
            InboxAction::RecordThis { path } => {
                assert_eq!(path, PathBuf::from("/tmp/clip.wav"))
            }
            _ => panic!("expected RecordThis"),
        }
    }

    #[test]
    fn archive_inbox_entry_moves_to_processed() {
        let dir = tempfile::tempdir().unwrap();
        let root = ensure_dirs(dir.path()).unwrap();
        let p = root.join(INBOX_DIRNAME).join("a.json");
        std::fs::write(&p, "{}").unwrap();
        let target = archive_inbox_entry(&p).unwrap();
        assert!(!p.exists());
        assert!(target.exists());
        assert!(target.parent().unwrap().ends_with(".processed"));
    }
}
