//! Markdown notes store. Each note is a single `.md` file in the notes
//! directory. The on-disk file is canonical; the in-memory `NotesStore` is
//! an index for the UI.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Local};
use tracing::warn;

#[derive(Clone, Debug)]
pub struct Note {
    pub path: PathBuf,
    pub title: String,
    pub modified: Option<SystemTime>,
    pub size_bytes: u64,
}

impl Note {
    pub fn modified_label(&self) -> String {
        self.modified
            .map(|m| {
                let dt: DateTime<Local> = m.into();
                dt.format("%b %-d, %H:%M").to_string()
            })
            .unwrap_or_else(|| "—".into())
    }
}

#[derive(Default)]
pub struct NotesStore {
    pub dir: PathBuf,
    pub notes: Vec<Note>,
}

impl NotesStore {
    pub fn load(dir: &Path) -> Self {
        let mut store = Self {
            dir: dir.to_path_buf(),
            notes: Vec::new(),
        };
        store.refresh();
        store
    }

    pub fn refresh(&mut self) {
        if let Err(e) = std::fs::create_dir_all(&self.dir) {
            warn!(error = %e, path = %self.dir.display(), "could not create notes dir");
            return;
        }
        let mut notes = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let meta = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let title = path
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "untitled".into());
                notes.push(Note {
                    path,
                    title,
                    modified: meta.modified().ok(),
                    size_bytes: meta.len(),
                });
            }
        }
        notes.sort_by(|a, b| match (b.modified, a.modified) {
            (Some(bm), Some(am)) => bm.cmp(&am),
            _ => b.title.cmp(&a.title),
        });
        self.notes = notes;
    }

    pub fn read(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    pub fn write(&self, path: &Path, contents: &str) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("md.tmp");
        std::fs::write(&tmp, contents)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn create_new(&self) -> std::io::Result<Note> {
        std::fs::create_dir_all(&self.dir)?;
        let now: DateTime<Local> = SystemTime::now().into();
        let base = now.format("%Y-%m-%d %H-%M").to_string();
        let mut title = base.clone();
        let mut i = 2;
        loop {
            let candidate = self.dir.join(format!("{}.md", title));
            if !candidate.exists() {
                let contents = format!("# Untitled\n\nCreated {}\n", now.format("%Y-%m-%d %H:%M"));
                self.write(&candidate, &contents)?;
                return Ok(Note {
                    path: candidate,
                    title,
                    modified: Some(SystemTime::now()),
                    size_bytes: contents.len() as u64,
                });
            }
            title = format!("{base} ({i})");
            i += 1;
        }
    }

    pub fn rename(&self, path: &Path, new_title: &str) -> std::io::Result<PathBuf> {
        let sanitized = new_title
            .chars()
            .map(|c| if c == '/' || c == '\\' { '-' } else { c })
            .collect::<String>();
        let target = self.dir.join(format!("{}.md", sanitized.trim()));
        if target == path {
            return Ok(target);
        }
        std::fs::rename(path, &target)?;
        Ok(target)
    }

    pub fn delete(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }
}
