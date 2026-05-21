//! Task store. Persisted as JSON. Three statuses: To-do, Doing, Done.
//! Kept simple so we can iterate quickly: no due dates, no tags v0; add
//! them when the workflow demands it.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    Doing,
    Done,
}

impl TaskStatus {
    pub fn label(self) -> &'static str {
        match self {
            TaskStatus::Todo => "To-do",
            TaskStatus::Doing => "Doing",
            TaskStatus::Done => "Done",
        }
    }
    pub fn next(self) -> Self {
        match self {
            TaskStatus::Todo => TaskStatus::Doing,
            TaskStatus::Doing => TaskStatus::Done,
            TaskStatus::Done => TaskStatus::Todo,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            TaskStatus::Todo => TaskStatus::Done,
            TaskStatus::Doing => TaskStatus::Todo,
            TaskStatus::Done => TaskStatus::Doing,
        }
    }
    pub fn all() -> &'static [TaskStatus] {
        &[TaskStatus::Todo, TaskStatus::Doing, TaskStatus::Done]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            description: String::new(),
            status: TaskStatus::Todo,
            created_at: now,
            updated_at: now,
        }
    }
    pub fn created_label(&self) -> String {
        let local: DateTime<Local> = self.created_at.into();
        local.format("%b %-d").to_string()
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct TasksFile {
    tasks: Vec<Task>,
}

#[derive(Default)]
pub struct TaskStore {
    pub path: PathBuf,
    pub tasks: Vec<Task>,
    pub draft_title: String,
    pub editing_task: Option<Uuid>,
}

impl TaskStore {
    pub fn load(path: &Path) -> Self {
        let mut store = Self {
            path: path.to_path_buf(),
            ..Default::default()
        };
        store.reload();
        store
    }

    pub fn reload(&mut self) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let contents = match std::fs::read_to_string(&self.path) {
            Ok(s) => s,
            Err(_) => {
                self.tasks = Vec::new();
                return;
            }
        };
        match serde_json::from_str::<TasksFile>(&contents) {
            Ok(f) => self.tasks = f.tasks,
            Err(e) => {
                warn!(error = %e, "could not parse tasks file; starting fresh");
                self.tasks = Vec::new();
            }
        }
    }

    pub fn save(&self) {
        if let Some(parent) = self.path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!(error = %e, "could not create tasks dir");
                return;
            }
        }
        let file = TasksFile {
            tasks: self.tasks.clone(),
        };
        match serde_json::to_string_pretty(&file) {
            Ok(s) => {
                let tmp = self.path.with_extension("json.tmp");
                if let Err(e) = std::fs::write(&tmp, s) {
                    warn!(error = %e, "could not write tasks tmp file");
                    return;
                }
                if let Err(e) = std::fs::rename(&tmp, &self.path) {
                    warn!(error = %e, "could not move tasks file into place");
                }
            }
            Err(e) => warn!(error = %e, "could not serialize tasks"),
        }
    }

    pub fn add(&mut self, title: String) {
        if title.trim().is_empty() {
            return;
        }
        self.tasks.push(Task::new(title.trim().to_string()));
        self.save();
    }

    pub fn delete(&mut self, id: Uuid) {
        self.tasks.retain(|t| t.id != id);
        self.save();
    }

    pub fn move_to(&mut self, id: Uuid, status: TaskStatus) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
            t.status = status;
            t.updated_at = Utc::now();
            self.save();
        }
    }

    pub fn update_title(&mut self, id: Uuid, title: String) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
            t.title = title;
            t.updated_at = Utc::now();
            self.save();
        }
    }

    pub fn update_description(&mut self, id: Uuid, description: String) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
            t.description = description;
            t.updated_at = Utc::now();
            self.save();
        }
    }

    pub fn count(&self, status: TaskStatus) -> usize {
        self.tasks.iter().filter(|t| t.status == status).count()
    }

    pub fn by_status(&self, status: TaskStatus) -> impl Iterator<Item = &Task> {
        self.tasks.iter().filter(move |t| t.status == status)
    }
}
