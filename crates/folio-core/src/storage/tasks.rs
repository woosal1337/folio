use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{FolioError, Result};

#[non_exhaustive]
#[derive(Copy, Clone, Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[ts(export, export_to = "../../../src/shared/types/")]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Todo,

    Doing,

    Done,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct Task {
    pub id: String,

    pub title: String,

    pub status: TaskStatus,

    pub owner: Option<String>,

    pub due: Option<String>,

    pub notes: Option<String>,

    pub source_session_dir: Option<String>,

    pub source_session_label: Option<String>,

    pub agent_origin: bool,

    pub created_at: DateTime<Utc>,

    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub status: Option<TaskStatus>,
    pub owner: Option<String>,
    pub due: Option<String>,
    pub notes: Option<String>,
}

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

    pub fn get(&self, id: &str) -> Option<Task> {
        self.list().into_iter().find(|t| t.id == id)
    }

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

    pub fn update(&self, id: &str, patch: TaskUpdate) -> Result<Task> {
        let mut tasks = self.list();
        let Some(task) = tasks.iter_mut().find(|t| t.id == id) else {
            return Err(FolioError::Storage(format!("task {id} not found")));
        };
        if let Some(title) = patch.title {
            task.title = title;
        }
        if let Some(status) = patch.status {
            task.status = status;
        }

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

    pub fn set_status(&self, id: &str, status: TaskStatus) -> Result<Task> {
        self.update(
            id,
            TaskUpdate {
                status: Some(status),
                ..TaskUpdate::default()
            },
        )
    }

    fn save(&self, tasks: &[Task]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                FolioError::Storage(format!(
                    "could not create tasks dir {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let json = serde_json::to_string_pretty(tasks)
            .map_err(|e| FolioError::Storage(format!("could not serialize tasks: {e}")))?;
        crate::storage::atomic_write::atomic_write(&self.path, json.as_bytes())?;
        Ok(())
    }
}

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
