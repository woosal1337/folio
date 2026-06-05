use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AttuneError, Result};
use crate::storage::atomic_write::atomic_write;

const CHATS_DIR: &str = ".attune/chats";

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ChatMessageRec {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ChatThread {
    pub id: String,
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_dir: Option<String>,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Vec<ChatMessageRec>,
}

fn chats_dir(output_dir: &Path) -> PathBuf {
    output_dir.join(CHATS_DIR)
}

fn thread_path(output_dir: &Path, id: &str) -> PathBuf {
    let safe: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    chats_dir(output_dir).join(format!("{safe}.json"))
}

pub fn list_threads(
    output_dir: &Path,
    scope: Option<&str>,
    session_dir: Option<&str>,
) -> Vec<ChatThread> {
    let dir = chats_dir(output_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<ChatThread> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(thread) = serde_json::from_str::<ChatThread>(&raw) else {
            continue;
        };
        if let Some(s) = scope {
            if thread.scope != s {
                continue;
            }
        }
        if let Some(sd) = session_dir {
            if thread.session_dir.as_deref() != Some(sd) {
                continue;
            }
        }
        out.push(thread);
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    out
}

pub fn save_thread(output_dir: &Path, thread: &ChatThread) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(thread)
        .map_err(|e| AttuneError::Storage(format!("serialize chat thread: {e}")))?;
    atomic_write(&thread_path(output_dir, &thread.id), &bytes)
}

pub fn delete_thread(output_dir: &Path, id: &str) -> Result<()> {
    let path = thread_path(output_dir, id);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AttuneError::Storage(format!(
            "delete chat thread {}: {e}",
            path.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thread(id: &str, scope: &str, updated: &str) -> ChatThread {
        ChatThread {
            id: id.to_string(),
            scope: scope.to_string(),
            session_dir: None,
            title: format!("Thread {id}"),
            created_at: "2026-05-29T00:00:00Z".to_string(),
            updated_at: updated.to_string(),
            messages: vec![ChatMessageRec {
                role: "user".to_string(),
                content: "hi".to_string(),
            }],
        }
    }

    #[test]
    fn save_then_list_roundtrips() {
        let tmp = tempfile::tempdir().unwrap();
        save_thread(
            tmp.path(),
            &thread("abc", "library", "2026-05-29T01:00:00Z"),
        )
        .unwrap();
        let listed = list_threads(tmp.path(), None, None);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "abc");
        assert_eq!(listed[0].messages.len(), 1);
    }

    #[test]
    fn list_sorts_newest_first() {
        let tmp = tempfile::tempdir().unwrap();
        save_thread(
            tmp.path(),
            &thread("old", "library", "2026-05-29T01:00:00Z"),
        )
        .unwrap();
        save_thread(
            tmp.path(),
            &thread("new", "library", "2026-05-29T02:00:00Z"),
        )
        .unwrap();
        let listed = list_threads(tmp.path(), None, None);
        assert_eq!(listed[0].id, "new");
        assert_eq!(listed[1].id, "old");
    }

    #[test]
    fn scope_and_session_filters_apply() {
        let tmp = tempfile::tempdir().unwrap();
        save_thread(
            tmp.path(),
            &thread("lib", "library", "2026-05-29T01:00:00Z"),
        )
        .unwrap();
        let mut note = thread("note", "note", "2026-05-29T02:00:00Z");
        note.session_dir = Some("/tmp/Attune/2026-a".to_string());
        save_thread(tmp.path(), &note).unwrap();

        assert_eq!(list_threads(tmp.path(), Some("library"), None).len(), 1);
        let note_hits = list_threads(tmp.path(), Some("note"), Some("/tmp/Attune/2026-a"));
        assert_eq!(note_hits.len(), 1);
        assert_eq!(note_hits[0].id, "note");
        assert!(list_threads(tmp.path(), Some("note"), Some("/other")).is_empty());
    }

    #[test]
    fn save_overwrites_same_id() {
        let tmp = tempfile::tempdir().unwrap();
        save_thread(tmp.path(), &thread("x", "library", "2026-05-29T01:00:00Z")).unwrap();
        let mut updated = thread("x", "library", "2026-05-29T03:00:00Z");
        updated.title = "Renamed".to_string();
        save_thread(tmp.path(), &updated).unwrap();
        let listed = list_threads(tmp.path(), None, None);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].title, "Renamed");
    }

    #[test]
    fn delete_removes_thread() {
        let tmp = tempfile::tempdir().unwrap();
        save_thread(
            tmp.path(),
            &thread("gone", "library", "2026-05-29T01:00:00Z"),
        )
        .unwrap();
        delete_thread(tmp.path(), "gone").unwrap();
        assert!(list_threads(tmp.path(), None, None).is_empty());

        delete_thread(tmp.path(), "gone").unwrap();
    }

    #[test]
    fn id_is_sanitised_against_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let mut t = thread("../escape", "library", "2026-05-29T01:00:00Z");
        t.id = "../escape".to_string();
        save_thread(tmp.path(), &t).unwrap();

        assert!(chats_dir(tmp.path()).join("escape.json").exists());
    }
}
