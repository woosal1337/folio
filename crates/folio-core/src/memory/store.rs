use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use parking_lot::Mutex;
use tracing::{debug, info};
use uuid::Uuid;

use crate::error::Result;
use crate::memory::index::MemoryIndex;
use crate::memory::page::write_page;
use crate::memory::types::{Memory, MemoryKind, MemoryQuery, MemoryUpdate, NewMemory};

#[derive(Debug, Clone)]
pub enum CreateOutcome {
    Added(Memory),

    Updated(Box<UpdatedMemory>),

    NoOp(Memory),
}

#[derive(Debug, Clone)]
pub struct UpdatedMemory {
    pub previous: Memory,
    pub current: Memory,
}

impl CreateOutcome {
    pub fn into_memory(self) -> Memory {
        match self {
            CreateOutcome::Added(m) => m,
            CreateOutcome::Updated(u) => u.current,
            CreateOutcome::NoOp(m) => m,
        }
    }
}

pub struct MemoryStore {
    dir: PathBuf,
    index: Arc<Mutex<MemoryIndex>>,
}

impl MemoryStore {
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();
        let index = MemoryIndex::open(&dir)?;
        Ok(Self {
            dir,
            index: Arc::new(Mutex::new(index)),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn list(&self, query: &MemoryQuery) -> Result<Vec<Memory>> {
        let index = self.index.lock();
        let mut rows = if let Some(q) = query.query.as_deref().filter(|s| !s.trim().is_empty()) {
            index.search(
                q,
                None,
                &query.kinds,
                query.limit.unwrap_or(50),
                query.include_archived,
            )?
        } else {
            let mut all = index.list_all(query.include_archived)?;
            if !query.kinds.is_empty() {
                all.retain(|m| query.kinds.contains(&m.kind));
            }
            all
        };
        if let Some(lim) = query.limit {
            rows.truncate(lim);
        }
        Ok(rows)
    }

    pub fn get(&self, id: &str) -> Result<Option<Memory>> {
        self.index.lock().get(id)
    }

    pub fn search(
        &self,
        query: &str,
        embedding: Option<&[f32]>,
        kinds: &[MemoryKind],
        limit: usize,
    ) -> Result<Vec<Memory>> {
        self.index
            .lock()
            .search(query, embedding, kinds, limit, false)
    }

    pub fn always_inject_set(&self, max_per_bucket: usize) -> Result<Vec<Memory>> {
        let index = self.index.lock();
        let all = index.list_all(false)?;
        let mut pinned: Vec<Memory> = all.iter().filter(|m| m.pinned).cloned().collect();
        let mut identity: Vec<Memory> = all
            .iter()
            .filter(|m| {
                !m.pinned
                    && m.kind == MemoryKind::Claim
                    && m.key
                        .as_deref()
                        .map(|k| k.starts_with("user."))
                        .unwrap_or(false)
            })
            .cloned()
            .collect();
        let mut prefs: Vec<Memory> = all
            .iter()
            .filter(|m| !m.pinned && m.kind == MemoryKind::Pref)
            .cloned()
            .collect();
        let mut projects: Vec<Memory> = all
            .iter()
            .filter(|m| {
                !m.pinned
                    && m.kind == MemoryKind::Claim
                    && m.key
                        .as_deref()
                        .map(|k| k.starts_with("project."))
                        .unwrap_or(false)
            })
            .cloned()
            .collect();
        identity.truncate(max_per_bucket);
        prefs.truncate(max_per_bucket);
        projects.truncate(max_per_bucket);
        let mut out = Vec::new();
        out.append(&mut pinned);
        out.append(&mut identity);
        out.append(&mut prefs);
        out.append(&mut projects);
        Ok(out)
    }

    pub fn create(&self, new: NewMemory) -> Result<CreateOutcome> {
        let now = Utc::now();
        let mut memory = Memory {
            id: Uuid::now_v7().to_string(),
            kind: new.kind,
            key: new.key.clone(),
            content: new.content.trim().to_string(),
            evidence: new.evidence,
            confidence: new.confidence.clamp(0.0, 1.0),
            tags: new.tags,
            source_session_dir: new.source_session_dir,
            source_session_label: new.source_session_label,
            valid_from: now,
            valid_until: None,
            supersedes_id: None,
            pinned: false,
            created_at: now,
            updated_at: now,
            extras: std::collections::BTreeMap::new(),
        };

        if memory.content.is_empty() {
            return Err(crate::error::FolioError::Storage(
                "memory content is empty".into(),
            ));
        }

        if memory.kind.is_keyed() {
            if let Some(key) = memory.key.as_deref() {
                let existing = {
                    let index = self.index.lock();
                    index.current_for_key(memory.kind, key)?
                };
                if let Some(prev) = existing {
                    if prev.content.trim() == memory.content {
                        debug!(key, "create: noop (identical content)");
                        return Ok(CreateOutcome::NoOp(prev));
                    }

                    memory.supersedes_id = Some(prev.id.clone());
                    let mut superseded = prev.clone();
                    superseded.valid_until = Some(now);
                    superseded.updated_at = now;
                    self.write_through(&superseded)?;
                    self.write_through(&memory)?;
                    info!(key, prev_id = %prev.id, new_id = %memory.id, "memory updated via supersede");
                    return Ok(CreateOutcome::Updated(Box::new(UpdatedMemory {
                        previous: superseded,
                        current: memory,
                    })));
                }
            }
        }

        self.write_through(&memory)?;
        info!(id = %memory.id, kind = %memory.kind.as_str(), "memory added");
        Ok(CreateOutcome::Added(memory))
    }

    pub fn update(&self, id: &str, patch: MemoryUpdate) -> Result<Memory> {
        let mut current = self
            .get(id)?
            .ok_or_else(|| crate::error::FolioError::Storage(format!("memory {id} not found")))?;
        if let Some(content) = patch.content {
            current.content = content.trim().to_string();
        }
        if let Some(key) = patch.key {
            current.key = if key.is_empty() { None } else { Some(key) };
        }
        if let Some(evidence) = patch.evidence {
            current.evidence = if evidence.is_empty() {
                None
            } else {
                Some(evidence)
            };
        }
        if let Some(tags) = patch.tags {
            current.tags = tags;
        }
        if let Some(pinned) = patch.pinned {
            current.pinned = pinned;
        }
        current.updated_at = Utc::now();
        self.write_through(&current)?;
        info!(id = %current.id, "memory updated");
        Ok(current)
    }

    fn write_through(&self, memory: &Memory) -> Result<()> {
        write_page(&self.dir, memory)?;
        self.index.lock().upsert(memory, None)?;
        Ok(())
    }

    pub fn upsert_with_embedding(&self, memory: &Memory, embedding: &[f32]) -> Result<()> {
        write_page(&self.dir, memory)?;
        self.index.lock().upsert(memory, Some(embedding))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store() -> (TempDir, MemoryStore) {
        let dir = TempDir::new().unwrap();
        let store = MemoryStore::open(dir.path()).unwrap();
        (dir, store)
    }

    fn nm(kind: MemoryKind, key: Option<&str>, content: &str) -> NewMemory {
        NewMemory {
            kind,
            key: key.map(String::from),
            content: content.into(),
            evidence: None,
            confidence: 0.9,
            tags: vec![],
            source_session_dir: None,
            source_session_label: None,
        }
    }

    #[test]
    fn add_path_creates_memory_and_file() {
        let (dir, s) = store();
        let outcome = s
            .create(nm(MemoryKind::Claim, Some("user.company"), "Folio"))
            .unwrap();
        match outcome {
            CreateOutcome::Added(m) => {
                assert_eq!(m.content, "Folio");
                let file_count = std::fs::read_dir(dir.path())
                    .unwrap()
                    .filter(|e| {
                        e.as_ref()
                            .ok()
                            .and_then(|e| e.path().extension().map(|x| x == "md"))
                            .unwrap_or(false)
                    })
                    .count();
                assert_eq!(file_count, 1);
            }
            other => panic!("expected Added, got {other:?}"),
        }
    }

    #[test]
    fn second_create_same_key_supersedes() {
        let (_dir, s) = store();
        let a = s
            .create(nm(MemoryKind::Claim, Some("user.company"), "Chele"))
            .unwrap()
            .into_memory();
        let outcome = s
            .create(nm(MemoryKind::Claim, Some("user.company"), "Folio"))
            .unwrap();
        match outcome {
            CreateOutcome::Updated(u) => {
                assert_eq!(u.previous.id, a.id);
                assert!(u.previous.valid_until.is_some());
                assert_eq!(u.current.supersedes_id, Some(a.id));
                assert_eq!(u.current.content, "Folio");
            }
            other => panic!("expected Updated, got {other:?}"),
        }

        let current = s
            .list(&MemoryQuery {
                kinds: vec![MemoryKind::Claim],
                ..Default::default()
            })
            .unwrap();
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].content, "Folio");
    }

    #[test]
    fn identical_content_is_noop() {
        let (_dir, s) = store();
        s.create(nm(MemoryKind::Claim, Some("user.company"), "Folio"))
            .unwrap();
        let outcome = s
            .create(nm(MemoryKind::Claim, Some("user.company"), "Folio"))
            .unwrap();
        assert!(matches!(outcome, CreateOutcome::NoOp(_)));
    }

    #[test]
    fn observe_does_not_supersede() {
        let (_dir, s) = store();
        s.create(nm(MemoryKind::Observe, None, "first")).unwrap();
        s.create(nm(MemoryKind::Observe, None, "second")).unwrap();
        let listed = s
            .list(&MemoryQuery {
                kinds: vec![MemoryKind::Observe],
                ..Default::default()
            })
            .unwrap();
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn always_inject_set_includes_identity_pref_project_and_pinned() {
        let (_dir, s) = store();
        s.create(nm(MemoryKind::Claim, Some("user.name"), "Ege"))
            .unwrap();
        s.create(nm(MemoryKind::Pref, Some("ui.theme"), "dark"))
            .unwrap();
        s.create(nm(
            MemoryKind::Claim,
            Some("project.folio"),
            "shipping memory v1",
        ))
        .unwrap();
        let extra = s
            .create(nm(MemoryKind::Observe, None, "random"))
            .unwrap()
            .into_memory();
        s.update(
            &extra.id,
            MemoryUpdate {
                pinned: Some(true),
                ..MemoryUpdate::default()
            },
        )
        .unwrap();
        let set = s.always_inject_set(5).unwrap();
        let contents: Vec<_> = set.iter().map(|m| m.content.as_str()).collect();
        assert!(contents.contains(&"Ege"));
        assert!(contents.contains(&"dark"));
        assert!(contents.contains(&"shipping memory v1"));
        assert!(contents.contains(&"random"));
    }

    #[test]
    fn search_returns_only_current_by_default() {
        let (_dir, s) = store();
        s.create(nm(MemoryKind::Claim, Some("user.company"), "Chele"))
            .unwrap();
        s.create(nm(MemoryKind::Claim, Some("user.company"), "Folio"))
            .unwrap();
        let hits = s.search("company", None, &[], 10).unwrap();
        assert_eq!(hits.len(), 1, "should only return the current memory");
        assert_eq!(hits[0].content, "Folio");
    }
}
