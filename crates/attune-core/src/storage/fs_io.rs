//! Filesystem outbox + inbox under `<memory_dir>/.attune/`.
//!
//! - **outbox/**: Attune writes one JSON file per lifecycle event
//!   (recording.finished, task.created, …) so external tools can
//!   pick them up by polling a dir or running an fs-watcher. Same
//!   shape as the webhook payload (v2 #079) for parity.
//! - **inbox/**: External tools drop `record-this.json` /
//!   `start-recording.json` / etc. into this dir; Attune scans on
//!   demand (or every N seconds — follow-up) and executes the
//!   matching action.
//!
//! Stable file format is the contract: any tool that learns to
//! write JSON in these shapes integrates forever. v2 finding 073
//! / GET-75.
//!
//! For MVP we ship: outbox write helper + inbox scan helper +
//! Tauri command to list pending inbox entries. An auto-execute
//! watcher is the natural follow-up.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{AttuneError, Result};

const OUTBOX_DIRNAME: &str = "outbox";
const INBOX_DIRNAME: &str = "inbox";

/// Resolve the `.attune/` root under the memory dir, creating it
/// (and its outbox + inbox children) if missing.
pub fn ensure_dirs(memory_dir: &Path) -> Result<PathBuf> {
    let root = memory_dir.join(".attune");
    for sub in ["", OUTBOX_DIRNAME, INBOX_DIRNAME] {
        let path = if sub.is_empty() {
            root.clone()
        } else {
            root.join(sub)
        };
        fs::create_dir_all(&path).map_err(|e| {
            AttuneError::Storage(format!("could not create {}: {e}", path.display()))
        })?;
    }
    Ok(root)
}

/// Internal-only outbox shape — we don't expose this across IPC since
/// the data field is arbitrary JSON. External tools read the JSON
/// files directly off disk; that's the whole contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub id: String,
    pub topic: String,
    pub emitted_at: String,
    pub data: serde_json::Value,
}

/// Append one event to the outbox. Filename pattern is
/// `<utc-iso>_<topic>_<short-uuid>.json` so chronological + topic
/// browsing in the shell stays cheap.
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
        .map_err(|e| AttuneError::Storage(format!("outbox serialize: {e}")))?;
    fs::write(&path, body)
        .map_err(|e| AttuneError::Storage(format!("outbox write {}: {e}", path.display())))?;
    Ok(path)
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct InboxEntry {
    /// Absolute path on disk; the UI uses this both as a stable
    /// React key and to surface 'reveal in Finder'.
    pub path: PathBuf,
    /// The file's basename, with the `.json` extension stripped.
    pub name: String,
    /// Bytes — useful when surfacing 'looks empty' empty-state
    /// without re-reading the file.
    pub bytes: u64,
    /// Modified time as RFC-3339 (best effort; empty when the
    /// platform doesn't supply it).
    pub modified_at: String,
}

/// List pending entries in the inbox, newest first. Files that
/// don't end in `.json` are silently skipped so the user can drop
/// `notes.md` in there without it showing up as a pending command.
pub fn list_inbox(memory_dir: &Path) -> Vec<InboxEntry> {
    let inbox = memory_dir.join(".attune").join(INBOX_DIRNAME);
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

/// Mark an inbox entry as handled by moving it to `inbox/.processed/`
/// with the same filename. Idempotent.
pub fn archive_inbox_entry(path: &Path) -> Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| AttuneError::Storage("inbox entry has no parent".to_string()))?;
    let processed = parent.join(".processed");
    fs::create_dir_all(&processed).map_err(|e| {
        AttuneError::Storage(format!(
            "could not create processed dir {}: {e}",
            processed.display()
        ))
    })?;
    let target = processed.join(
        path.file_name()
            .ok_or_else(|| AttuneError::Storage("inbox entry has no filename".into()))?,
    );
    fs::rename(path, &target).map_err(|e| {
        AttuneError::Storage(format!(
            "could not archive inbox entry {}: {e}",
            path.display()
        ))
    })?;
    Ok(target)
}

/// Discriminated-union shape Attune knows how to act on. Unknown
/// `kind` values are surfaced as Other so the UI can show them
/// without exploding.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum InboxAction {
    #[serde(rename = "start-recording")]
    StartRecording,
    #[serde(rename = "stop-recording")]
    StopRecording,
    #[serde(rename = "record-this")]
    RecordThis {
        /// Absolute path to an audio file the user wants ingested +
        /// transcribed. Same contract as the drag-onto-dock-icon path
        /// from GET-103.
        path: PathBuf,
    },
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
        assert!(path.starts_with(dir.path().join(".attune").join(OUTBOX_DIRNAME)));
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
