//! Persisted to-do list backing the Tasks kanban + the
//! `create_task` tool exposed to LLM agents.
//!
//! Tasks live in a single JSON array file at `settings.tasks_path`
//! (default `~/Documents/Attune/Tasks/tasks.json`). One file keeps
//! the format simple, makes "show me everything" cheap, and avoids
//! the directory-of-files pattern's pathological behaviour on
//! mid-write crashes. We pay for it on writes: every mutation
//! re-serialises the whole list. With realistic task counts (low
//! thousands at most), that's well under a millisecond.
//!
//! The store is intentionally not a singleton — `TaskStore::new` is
//! cheap and the Tauri command layer instantiates one per call. That
//! keeps the API and the test surface tiny.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{AttuneError, Result};

/// Kanban column / lifecycle state of a task.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../../src/shared/types/")]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    Doing,
    Done,
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Todo
    }
}

/// A single to-do item. Always has an id + title + status; everything
/// else is optional metadata so the model can fill in what it knows
/// without us rejecting a tool call for a missing owner.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct Task {
    /// UUID v4 string. Generated server-side so the frontend can't
    /// collide.
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    /// Free-form owner ("Ege", "design team", "@alice"). The schema
    /// does not enforce a particular handle format because meeting
    /// transcripts use whatever conventions the speakers prefer.
    pub owner: Option<String>,
    /// Free-form due date string. We don't parse this server-side —
    /// the model produces "Friday", "next sprint", "2026-06-01" and
    /// users type whatever they want. The kanban renders it as-is.
    pub due: Option<String>,
    /// Optional extra context: "see point 3 in the deck", "blocked
    /// on legal review", etc.
    pub notes: Option<String>,
    /// Session directory of the recording this task was extracted
    /// from. Lets the UI deep-link back to the editor and surface a
    /// "see source" button. None for manually-created tasks.
    pub source_session_dir: Option<String>,
    /// Human-readable label of the source recording (the trailing
    /// component of session_dir) so we don't have to re-derive it in
    /// every renderer.
    pub source_session_label: Option<String>,
    /// True when an agent created this task via `create_task`. Lets
    /// the UI mark agent-origin cards with a sparkle so the user can
    /// scan the board and tell at a glance what came from a meeting.
    pub agent_origin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Partial update sent through `update_task`. Each field is
/// `Option<Option<T>>` so the wire format can distinguish
/// "not present in the patch" (None) from "explicitly clear this
/// nullable field" (Some(None)) for nullable fields. For now we keep
/// it simple — `None` means "leave alone", `Some(value)` means set.
/// Setting a nullable field to None via update is uncommon enough
/// that we don't model "clear" separately; users delete the task
/// instead.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub status: Option<TaskStatus>,
    pub owner: Option<String>,
    pub due: Option<String>,
    pub notes: Option<String>,
}

/// File-backed CRUD store for [`Task`]s.
pub struct TaskStore {
    path: PathBuf,
}

impl TaskStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load all tasks. Missing file → empty list; malformed file →
    /// warn + empty list (so a corrupt tasks.json never blocks the
    /// app from starting). Callers that need to surface a load error
    /// to the user should re-read the file directly.
    pub fn list(&self) -> Vec<Task> {
        match fs::read_to_string(&self.path) {
            Ok(contents) if contents.trim().is_empty() => {
                debug!(path = %self.path.display(), "tasks file empty, returning []");
                Vec::new()
            }
            Ok(contents) => match serde_json::from_str::<Vec<Task>>(&contents) {
                Ok(tasks) => {
                    debug!(path = %self.path.display(), count = tasks.len(), "tasks loaded");
                    tasks
                }
                Err(e) => {
                    warn!(
                        path = %self.path.display(),
                        error = %e,
                        "tasks file is malformed; returning []",
                    );
                    Vec::new()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!(path = %self.path.display(), "no tasks file; returning []");
                Vec::new()
            }
            Err(e) => {
                warn!(
                    path = %self.path.display(),
                    error = %e,
                    "could not read tasks file; returning []",
                );
                Vec::new()
            }
        }
    }

    /// Fetch a single task by id. None if missing.
    pub fn get(&self, id: &str) -> Option<Task> {
        self.list().into_iter().find(|t| t.id == id)
    }

    /// Append a new task. The store generates the id + timestamps so
    /// the caller can't accidentally collide ids.
    pub fn create(&self, new_task: NewTask) -> Result<Task> {
        let now = Utc::now();
        let task = Task {
            id: Uuid::new_v4().to_string(),
            title: new_task.title,
            status: new_task.status.unwrap_or_default(),
            owner: new_task.owner,
            due: new_task.due,
            notes: new_task.notes,
            source_session_dir: new_task.source_session_dir,
            source_session_label: new_task.source_session_label,
            agent_origin: new_task.agent_origin,
            created_at: now,
            updated_at: now,
        };
        let mut tasks = self.list();
        tasks.push(task.clone());
        self.save(&tasks)?;
        info!(id = %task.id, title = %task.title, "task created");
        Ok(task)
    }

    /// Apply a patch to a task. Returns the updated task or an error
    /// when the id is unknown.
    pub fn update(&self, id: &str, patch: TaskUpdate) -> Result<Task> {
        let mut tasks = self.list();
        let Some(task) = tasks.iter_mut().find(|t| t.id == id) else {
            return Err(AttuneError::Storage(format!("task {id} not found")));
        };
        if let Some(title) = patch.title {
            task.title = title;
        }
        if let Some(status) = patch.status {
            task.status = status;
        }
        // Patch semantics for nullable fields: a Some(value) sets it,
        // None leaves it alone. To clear, send an empty string — the
        // UI's edit form treats "" as "clear".
        if let Some(owner) = patch.owner {
            task.owner = if owner.is_empty() { None } else { Some(owner) };
        }
        if let Some(due) = patch.due {
            task.due = if due.is_empty() { None } else { Some(due) };
        }
        if let Some(notes) = patch.notes {
            task.notes = if notes.is_empty() { None } else { Some(notes) };
        }
        task.updated_at = Utc::now();
        let updated = task.clone();
        self.save(&tasks)?;
        info!(id = %updated.id, "task updated");
        Ok(updated)
    }

    /// Delete a task by id. Idempotent — deleting an unknown id is a
    /// no-op + success, matching how UI delete buttons should feel.
    pub fn delete(&self, id: &str) -> Result<()> {
        let mut tasks = self.list();
        let before = tasks.len();
        tasks.retain(|t| t.id != id);
        if tasks.len() == before {
            debug!(id = %id, "delete: task not found, no-op");
            return Ok(());
        }
        self.save(&tasks)?;
        info!(id = %id, "task deleted");
        Ok(())
    }

    /// Convenience wrapper used by the kanban's drag-and-drop.
    pub fn set_status(&self, id: &str, status: TaskStatus) -> Result<Task> {
        self.update(
            id,
            TaskUpdate {
                status: Some(status),
                ..TaskUpdate::default()
            },
        )
    }

    /// Atomic write of the full task list. Creates the parent dir
    /// if missing; writes to a sibling temp file then renames so a
    /// crash mid-write cannot corrupt the on-disk file.
    fn save(&self, tasks: &[Task]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AttuneError::Storage(format!(
                    "could not create tasks dir {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let json = serde_json::to_string_pretty(tasks)
            .map_err(|e| AttuneError::Storage(format!("could not serialize tasks: {e}")))?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| {
            AttuneError::Storage(format!(
                "could not write tasks temp file {}: {e}",
                tmp.display()
            ))
        })?;
        fs::rename(&tmp, &self.path).map_err(|e| {
            AttuneError::Storage(format!(
                "could not finalize tasks file {}: {e}",
                self.path.display()
            ))
        })?;
        Ok(())
    }
}

/// Constructor payload for [`TaskStore::create`]. Everything except
/// the title is optional so the kanban's inline composer can create
/// a task with just a title and the `create_task` tool can omit any
/// field the model didn't pick up.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct NewTask {
    pub title: String,
    pub status: Option<TaskStatus>,
    pub owner: Option<String>,
    pub due: Option<String>,
    pub notes: Option<String>,
    pub source_session_dir: Option<String>,
    pub source_session_label: Option<String>,
    #[serde(default)]
    pub agent_origin: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store() -> (TempDir, TaskStore) {
        let dir = TempDir::new().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        (dir, store)
    }

    #[test]
    fn list_returns_empty_when_file_missing() {
        let (_dir, store) = store();
        assert!(store.list().is_empty());
    }

    #[test]
    fn create_persists_and_round_trips() {
        let (_dir, store) = store();
        let task = store
            .create(NewTask {
                title: "Ship the kanban".into(),
                owner: Some("Ege".into()),
                ..NewTask::default()
            })
            .unwrap();
        let listed = store.list();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, task.id);
        assert_eq!(listed[0].owner.as_deref(), Some("Ege"));
        assert_eq!(listed[0].status, TaskStatus::Todo);
    }

    #[test]
    fn update_changes_title_and_status_and_bumps_updated_at() {
        let (_dir, store) = store();
        let original = store
            .create(NewTask {
                title: "draft".into(),
                ..NewTask::default()
            })
            .unwrap();
        // tiny sleep so updated_at moves
        std::thread::sleep(std::time::Duration::from_millis(2));
        let updated = store
            .update(
                &original.id,
                TaskUpdate {
                    title: Some("final".into()),
                    status: Some(TaskStatus::Doing),
                    ..TaskUpdate::default()
                },
            )
            .unwrap();
        assert_eq!(updated.title, "final");
        assert_eq!(updated.status, TaskStatus::Doing);
        assert!(updated.updated_at > original.updated_at);
    }

    #[test]
    fn update_empty_string_clears_nullable_fields() {
        let (_dir, store) = store();
        let t = store
            .create(NewTask {
                title: "x".into(),
                owner: Some("Ege".into()),
                due: Some("Friday".into()),
                ..NewTask::default()
            })
            .unwrap();
        let cleared = store
            .update(
                &t.id,
                TaskUpdate {
                    owner: Some(String::new()),
                    due: Some(String::new()),
                    ..TaskUpdate::default()
                },
            )
            .unwrap();
        assert!(cleared.owner.is_none());
        assert!(cleared.due.is_none());
    }

    #[test]
    fn delete_is_idempotent() {
        let (_dir, store) = store();
        let t = store
            .create(NewTask {
                title: "x".into(),
                ..NewTask::default()
            })
            .unwrap();
        store.delete(&t.id).unwrap();
        // Second delete must succeed without error.
        store.delete(&t.id).unwrap();
        assert!(store.list().is_empty());
    }

    #[test]
    fn set_status_round_trips() {
        let (_dir, store) = store();
        let t = store
            .create(NewTask {
                title: "x".into(),
                ..NewTask::default()
            })
            .unwrap();
        let moved = store.set_status(&t.id, TaskStatus::Done).unwrap();
        assert_eq!(moved.status, TaskStatus::Done);
        assert_eq!(store.get(&t.id).unwrap().status, TaskStatus::Done);
    }

    #[test]
    fn malformed_file_yields_empty_list_not_error() {
        let (dir, _store) = store();
        let path = dir.path().join("bad.json");
        fs::write(&path, "{ not json").unwrap();
        let store = TaskStore::new(path);
        assert!(store.list().is_empty());
    }

    #[test]
    fn save_creates_missing_parent_dir() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("a").join("b").join("tasks.json");
        let store = TaskStore::new(&nested);
        store
            .create(NewTask {
                title: "x".into(),
                ..NewTask::default()
            })
            .unwrap();
        assert!(nested.exists());
    }
}
