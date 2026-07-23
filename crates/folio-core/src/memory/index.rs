use std::path::{Path, PathBuf};
use std::sync::Once;

use rusqlite::ffi::sqlite3_auto_extension;
use rusqlite::{params, Connection, OptionalExtension};
use sqlite_vec::sqlite3_vec_init;
use tracing::{debug, info, warn};
use zerocopy::AsBytes;

use crate::error::{FolioError, Result};
use crate::memory::types::{Memory, MemoryKind};

pub const EMBEDDING_DIMS: usize = 3072;
const SCHEMA_VERSION: i64 = 1;

static REGISTER_VEC: Once = Once::new();

type AutoExtFn = unsafe extern "C" fn(
    *mut rusqlite::ffi::sqlite3,
    *mut *mut std::os::raw::c_char,
    *const rusqlite::ffi::sqlite3_api_routines,
) -> std::os::raw::c_int;

fn register_vec_extension_once() {
    REGISTER_VEC.call_once(|| unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute::<*const (), AutoExtFn>(
            sqlite3_vec_init as *const (),
        )));
    });
}

pub struct MemoryIndex {
    conn: Connection,
    db_path: PathBuf,
}

impl MemoryIndex {
    pub fn open(memory_dir: &Path) -> Result<Self> {
        register_vec_extension_once();
        std::fs::create_dir_all(memory_dir).map_err(|e| {
            FolioError::Storage(format!(
                "could not create memory dir {}: {e}",
                memory_dir.display()
            ))
        })?;
        let db_path = memory_dir.join(".index.sqlite");
        let conn = Connection::open(&db_path).map_err(|e| {
            FolioError::Storage(format!(
                "could not open memory index {}: {e}",
                db_path.display()
            ))
        })?;
        let mut index = Self { conn, db_path };
        index.init_schema()?;
        Ok(index)
    }

    fn init_schema(&mut self) -> Result<()> {
        let current: Option<i64> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|s| s.parse().ok());
        if current == Some(SCHEMA_VERSION) {
            return Ok(());
        }
        if current.is_some() {
            warn!(
                current = ?current,
                target = SCHEMA_VERSION,
                "memory index schema mismatch; wiping and rebuilding"
            );
            drop(std::mem::replace(
                &mut self.conn,
                Connection::open_in_memory()
                    .expect("in-memory SQLite should always open; infallible on all targets"),
            ));
            let _ = std::fs::remove_file(&self.db_path);
            self.conn = Connection::open(&self.db_path).map_err(|e| {
                FolioError::Storage(format!("reopen after schema wipe failed: {e}"))
            })?;
        }
        self.conn
            .execute_batch(&format!(
                r#"
CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS memories (
    id            TEXT PRIMARY KEY,
    kind          TEXT NOT NULL,
    key           TEXT,
    content       TEXT NOT NULL,
    evidence      TEXT,
    confidence    REAL NOT NULL,
    tags          TEXT NOT NULL,
    source_dir    TEXT,
    source_label  TEXT,
    valid_from    TEXT NOT NULL,
    valid_until   TEXT,
    supersedes_id TEXT,
    pinned        INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS memories_kind_current
  ON memories (kind, valid_until);
CREATE INDEX IF NOT EXISTS memories_key
  ON memories (key);
CREATE INDEX IF NOT EXISTS memories_pinned
  ON memories (pinned) WHERE pinned = 1;

-- FTS5 over title (the key), tags and body with weight columns.
-- Avid Brain's title 10x / tags 5x / body 1x weights produced the
-- "indistinguishable from embeddings under 100k events" result we
-- want to reproduce here.
CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    title,
    tags,
    body,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE VIRTUAL TABLE IF NOT EXISTS memory_vec USING vec0(
    embedding float[{dims}]
);

INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', '{version}');
"#,
                dims = EMBEDDING_DIMS,
                version = SCHEMA_VERSION,
            ))
            .map_err(|e| FolioError::Storage(format!("init schema failed: {e}")))?;
        Ok(())
    }

    pub fn upsert(&self, memory: &Memory, embedding: Option<&[f32]>) -> Result<()> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| FolioError::Storage(format!("begin tx: {e}")))?;

        let tags_csv = memory.tags.join(",");
        tx.execute(
            r#"
INSERT INTO memories
  (id, kind, key, content, evidence, confidence, tags, source_dir, source_label,
   valid_from, valid_until, supersedes_id, pinned, created_at, updated_at)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
ON CONFLICT(id) DO UPDATE SET
  kind = excluded.kind,
  key = excluded.key,
  content = excluded.content,
  evidence = excluded.evidence,
  confidence = excluded.confidence,
  tags = excluded.tags,
  source_dir = excluded.source_dir,
  source_label = excluded.source_label,
  valid_from = excluded.valid_from,
  valid_until = excluded.valid_until,
  supersedes_id = excluded.supersedes_id,
  pinned = excluded.pinned,
  created_at = excluded.created_at,
  updated_at = excluded.updated_at
"#,
            params![
                memory.id,
                memory.kind.as_str(),
                memory.key,
                memory.content,
                memory.evidence,
                memory.confidence as f64,
                tags_csv,
                memory.source_session_dir,
                memory.source_session_label,
                memory.valid_from.to_rfc3339(),
                memory.valid_until.as_ref().map(|t| t.to_rfc3339()),
                memory.supersedes_id,
                memory.pinned as i64,
                memory.created_at.to_rfc3339(),
                memory.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| FolioError::Storage(format!("memories upsert: {e}")))?;

        tx.execute(
            "DELETE FROM memories_fts WHERE rowid = (SELECT rowid FROM memories WHERE id = ?)",
            params![memory.id],
        )
        .ok();
        let rowid: i64 = tx
            .query_row(
                "SELECT rowid FROM memories WHERE id = ?",
                params![memory.id],
                |row| row.get(0),
            )
            .map_err(|e| FolioError::Storage(format!("rowid lookup: {e}")))?;
        let title = memory.key.as_deref().unwrap_or("");
        tx.execute(
            "INSERT INTO memories_fts (rowid, title, tags, body) VALUES (?, ?, ?, ?)",
            params![rowid, title, tags_csv, memory.content],
        )
        .map_err(|e| FolioError::Storage(format!("fts insert: {e}")))?;

        tx.execute("DELETE FROM memory_vec WHERE rowid = ?", params![rowid])
            .ok();
        if let Some(embedding) = embedding {
            if embedding.len() != EMBEDDING_DIMS {
                return Err(FolioError::Storage(format!(
                    "embedding dim mismatch: got {}, want {}",
                    embedding.len(),
                    EMBEDDING_DIMS,
                )));
            }
            tx.execute(
                "INSERT INTO memory_vec (rowid, embedding) VALUES (?, ?)",
                params![rowid, embedding.as_bytes()],
            )
            .map_err(|e| FolioError::Storage(format!("vec insert: {e}")))?;
        }
        tx.commit()
            .map_err(|e| FolioError::Storage(format!("commit: {e}")))?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| FolioError::Storage(format!("begin tx: {e}")))?;
        let rowid: Option<i64> = tx
            .query_row(
                "SELECT rowid FROM memories WHERE id = ?",
                params![id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| FolioError::Storage(format!("rowid lookup: {e}")))?;
        if let Some(rowid) = rowid {
            tx.execute("DELETE FROM memories_fts WHERE rowid = ?", params![rowid])
                .ok();
            tx.execute("DELETE FROM memory_vec WHERE rowid = ?", params![rowid])
                .ok();
            tx.execute("DELETE FROM memories WHERE id = ?", params![id])
                .map_err(|e| FolioError::Storage(format!("memories delete: {e}")))?;
        }
        tx.commit()
            .map_err(|e| FolioError::Storage(format!("commit: {e}")))?;
        Ok(())
    }

    pub fn list_all(&self, include_archived: bool) -> Result<Vec<Memory>> {
        let where_clause = if include_archived {
            ""
        } else {
            "WHERE valid_until IS NULL"
        };
        let sql = format!(
            r#"
SELECT id, kind, key, content, evidence, confidence, tags, source_dir, source_label,
       valid_from, valid_until, supersedes_id, pinned, created_at, updated_at
FROM memories
{where_clause}
ORDER BY created_at DESC
"#,
        );
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| FolioError::Storage(format!("prepare list: {e}")))?;
        let rows = stmt
            .query_map([], row_to_memory)
            .map_err(|e| FolioError::Storage(format!("query list: {e}")))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| FolioError::Storage(format!("collect list: {e}")))
    }

    pub fn get(&self, id: &str) -> Result<Option<Memory>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
SELECT id, kind, key, content, evidence, confidence, tags, source_dir, source_label,
       valid_from, valid_until, supersedes_id, pinned, created_at, updated_at
FROM memories WHERE id = ?
"#,
            )
            .map_err(|e| FolioError::Storage(format!("prepare get: {e}")))?;
        let mut rows = stmt
            .query_map(params![id], row_to_memory)
            .map_err(|e| FolioError::Storage(format!("query get: {e}")))?;
        match rows.next() {
            Some(Ok(m)) => Ok(Some(m)),
            Some(Err(e)) => Err(FolioError::Storage(format!("get row: {e}"))),
            None => Ok(None),
        }
    }

    pub fn current_for_key(&self, kind: MemoryKind, key: &str) -> Result<Option<Memory>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
SELECT id, kind, key, content, evidence, confidence, tags, source_dir, source_label,
       valid_from, valid_until, supersedes_id, pinned, created_at, updated_at
FROM memories
WHERE kind = ? AND key = ? AND valid_until IS NULL
ORDER BY created_at DESC
LIMIT 1
"#,
            )
            .map_err(|e| FolioError::Storage(format!("prepare current_for_key: {e}")))?;
        let mut rows = stmt
            .query_map(params![kind.as_str(), key], row_to_memory)
            .map_err(|e| FolioError::Storage(format!("query current_for_key: {e}")))?;
        match rows.next() {
            Some(Ok(m)) => Ok(Some(m)),
            Some(Err(e)) => Err(FolioError::Storage(format!("row: {e}"))),
            None => Ok(None),
        }
    }

    pub fn search(
        &self,
        query: &str,
        embedding: Option<&[f32]>,
        kinds: &[MemoryKind],
        limit: usize,
        include_archived: bool,
    ) -> Result<Vec<Memory>> {
        if query.trim().is_empty() && embedding.is_none() {
            let mut all = self.list_all(include_archived)?;
            all.truncate(limit);
            return Ok(all);
        }
        let mut scores: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

        if !query.trim().is_empty() {
            let fts_query = sanitize_fts_query(query);
            let mut stmt = self
                .conn
                .prepare(
                    r#"
SELECT m.id, bm25(memories_fts, 10.0, 5.0, 1.0) AS rank
FROM memories_fts
JOIN memories m ON m.rowid = memories_fts.rowid
WHERE memories_fts MATCH ?
ORDER BY rank
LIMIT ?
"#,
                )
                .map_err(|e| FolioError::Storage(format!("prepare fts: {e}")))?;
            let rows = stmt
                .query_map(params![fts_query, (limit * 4) as i64], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
                })
                .map_err(|e| FolioError::Storage(format!("query fts: {e}")))?;
            for (rank, row) in rows.enumerate() {
                let (id, _bm25) = row.map_err(|e| FolioError::Storage(format!("fts row: {e}")))?;
                let contribution = 1.0 / (60.0 + rank as f64);
                *scores.entry(id).or_insert(0.0) += contribution;
            }
        }

        if let Some(embedding) = embedding {
            if embedding.len() == EMBEDDING_DIMS {
                let mut stmt = self
                    .conn
                    .prepare(
                        r#"
SELECT m.id
FROM memory_vec v
JOIN memories m ON m.rowid = v.rowid
WHERE v.embedding MATCH ? AND k = ?
ORDER BY distance
"#,
                    )
                    .map_err(|e| FolioError::Storage(format!("prepare vec: {e}")))?;
                let rows = stmt
                    .query_map(params![embedding.as_bytes(), (limit * 4) as i64], |r| {
                        r.get::<_, String>(0)
                    })
                    .map_err(|e| FolioError::Storage(format!("query vec: {e}")))?;
                for (rank, row) in rows.enumerate() {
                    let id = row.map_err(|e| FolioError::Storage(format!("vec row: {e}")))?;
                    let contribution = 1.0 / (60.0 + rank as f64);
                    *scores.entry(id).or_insert(0.0) += contribution;
                }
            }
        }

        if scores.is_empty() {
            return Ok(Vec::new());
        }
        let mut ranked: Vec<(String, f64)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut out = Vec::with_capacity(limit);
        for (id, _) in ranked {
            if let Some(m) = self.get(&id)? {
                if !include_archived && !m.is_current() {
                    continue;
                }
                if !kinds.is_empty() && !kinds.contains(&m.kind) {
                    continue;
                }
                out.push(m);
                if out.len() >= limit {
                    break;
                }
            }
        }
        Ok(out)
    }

    pub fn rebuild_from(
        &mut self,
        memories: &[Memory],
        mut embeddings_for: impl FnMut(&str) -> Option<Vec<f32>>,
    ) -> Result<()> {
        info!(count = memories.len(), "rebuilding memory index from files");
        self.conn
            .execute_batch(
                r#"
DELETE FROM memories;
DELETE FROM memories_fts;
DELETE FROM memory_vec;
"#,
            )
            .map_err(|e| FolioError::Storage(format!("wipe index: {e}")))?;
        for m in memories {
            let emb = embeddings_for(&m.id);
            self.upsert(m, emb.as_deref())?;
        }
        debug!("rebuild complete");
        Ok(())
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

fn sanitize_fts_query(query: &str) -> String {
    let cleaned: String = query
        .chars()
        .map(|c| match c {
            '"' | '\'' | ':' | '*' | '(' | ')' | '-' | '+' => ' ',
            _ => c,
        })
        .collect();
    let parts: Vec<String> = cleaned
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|s| format!("{s}*"))
        .collect();
    if parts.is_empty() {
        "*".to_string()
    } else {
        parts.join(" ")
    }
}

fn row_to_memory(row: &rusqlite::Row<'_>) -> rusqlite::Result<Memory> {
    let kind_str: String = row.get("kind")?;
    let kind = MemoryKind::parse(&kind_str).unwrap_or_default();
    let tags_csv: String = row.get("tags")?;
    let tags: Vec<String> = if tags_csv.is_empty() {
        Vec::new()
    } else {
        tags_csv.split(',').map(|s| s.to_string()).collect()
    };
    let parse_dt = |s: String| -> rusqlite::Result<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(&s)
            .map(|d| d.with_timezone(&chrono::Utc))
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
    };
    Ok(Memory {
        id: row.get("id")?,
        kind,
        key: row.get("key")?,
        content: row.get("content")?,
        evidence: row.get("evidence")?,
        confidence: row.get::<_, f64>("confidence")? as f32,
        tags,
        source_session_dir: row.get("source_dir")?,
        source_session_label: row.get("source_label")?,
        valid_from: parse_dt(row.get("valid_from")?)?,
        valid_until: row
            .get::<_, Option<String>>("valid_until")?
            .map(parse_dt)
            .transpose()?,
        supersedes_id: row.get("supersedes_id")?,
        pinned: row.get::<_, i64>("pinned")? != 0,
        created_at: parse_dt(row.get("created_at")?)?,
        updated_at: parse_dt(row.get("updated_at")?)?,

        extras: std::collections::BTreeMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn make(id: &str, kind: MemoryKind, key: Option<&str>, content: &str) -> Memory {
        let now = Utc::now();
        Memory {
            id: id.to_string(),
            kind,
            key: key.map(String::from),
            content: content.to_string(),
            evidence: None,
            confidence: 0.9,
            tags: vec!["test".into()],
            source_session_dir: None,
            source_session_label: None,
            valid_from: now,
            valid_until: None,
            supersedes_id: None,
            pinned: false,
            created_at: now,
            updated_at: now,
            extras: std::collections::BTreeMap::new(),
        }
    }

    fn fresh2() -> (TempDir, MemoryIndex) {
        let dir = TempDir::new().unwrap();
        let idx = MemoryIndex::open(dir.path()).unwrap();
        (dir, idx)
    }

    #[test]
    fn open_creates_schema() {
        let (_dir, idx) = fresh2();
        assert!(idx.db_path().exists());
    }

    #[test]
    fn upsert_then_list_round_trips() {
        let (_dir, idx) = fresh2();
        let m = make(
            &Uuid::now_v7().to_string(),
            MemoryKind::Claim,
            Some("user.company"),
            "Folio",
        );
        idx.upsert(&m, None).unwrap();
        let listed = idx.list_all(false).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].content, "Folio");
    }

    #[test]
    fn current_for_key_returns_only_current() {
        let (_dir, idx) = fresh2();
        let mut old = make(
            &Uuid::now_v7().to_string(),
            MemoryKind::Claim,
            Some("user.company"),
            "Chele",
        );
        old.valid_until = Some(Utc::now());
        idx.upsert(&old, None).unwrap();
        let cur = make(
            &Uuid::now_v7().to_string(),
            MemoryKind::Claim,
            Some("user.company"),
            "Folio",
        );
        idx.upsert(&cur, None).unwrap();
        let got = idx
            .current_for_key(MemoryKind::Claim, "user.company")
            .unwrap()
            .unwrap();
        assert_eq!(got.content, "Folio");
    }

    #[test]
    fn fts_search_finds_by_title_and_body() {
        let (_dir, idx) = fresh2();
        let m = make(
            &Uuid::now_v7().to_string(),
            MemoryKind::Claim,
            Some("project.folio"),
            "shipping memory layer v1",
        );
        idx.upsert(&m, None).unwrap();
        let hits = idx.search("folio", None, &[], 5, false).unwrap();
        assert_eq!(hits.len(), 1);
        let hits = idx.search("memory", None, &[], 5, false).unwrap();
        assert_eq!(hits.len(), 1);
        let hits = idx.search("nonexistent", None, &[], 5, false).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn search_filters_by_kind() {
        let (_dir, idx) = fresh2();
        let a = make(
            &Uuid::now_v7().to_string(),
            MemoryKind::Claim,
            Some("k1"),
            "alpha",
        );
        let b = make(
            &Uuid::now_v7().to_string(),
            MemoryKind::Pref,
            Some("k2"),
            "alpha",
        );
        idx.upsert(&a, None).unwrap();
        idx.upsert(&b, None).unwrap();
        let only_claims = idx
            .search("alpha", None, &[MemoryKind::Claim], 5, false)
            .unwrap();
        assert_eq!(only_claims.len(), 1);
        assert_eq!(only_claims[0].kind, MemoryKind::Claim);
    }

    #[test]
    fn delete_removes_from_every_table() {
        let (_dir, idx) = fresh2();
        let m = make(
            &Uuid::now_v7().to_string(),
            MemoryKind::Observe,
            None,
            "throwaway",
        );
        idx.upsert(&m, None).unwrap();
        idx.delete(&m.id).unwrap();
        assert!(idx.list_all(true).unwrap().is_empty());
        assert!(idx
            .search("throwaway", None, &[], 5, true)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn rebuild_from_files_clears_then_reinserts() {
        let (_dir, mut idx) = fresh2();
        let a = make(
            &Uuid::now_v7().to_string(),
            MemoryKind::Claim,
            Some("k1"),
            "alpha",
        );
        let b = make(
            &Uuid::now_v7().to_string(),
            MemoryKind::Claim,
            Some("k2"),
            "beta",
        );
        idx.upsert(&a, None).unwrap();
        idx.rebuild_from(std::slice::from_ref(&b), |_| None)
            .unwrap();
        let listed = idx.list_all(false).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, b.id);
    }

    #[test]
    fn sanitize_strips_fts_specials() {
        assert_eq!(sanitize_fts_query("hello"), "hello*");
        assert_eq!(sanitize_fts_query("\"quoted\""), "quoted*");
        assert_eq!(sanitize_fts_query("a:b c-d"), "a* b* c* d*");
        assert_eq!(sanitize_fts_query("   "), "*");
    }
}
