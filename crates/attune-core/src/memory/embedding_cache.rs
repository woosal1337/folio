//! Embedding cache + shard policy. v2 finding 041 / GET-62.
//!
//! Two problems this module solves:
//!
//!   1. **Reindex re-charges OpenAI.** Today the reindex command
//!      walks every memory page and re-embeds them. Each pass spends
//!      a few cents per thousand pages. The cache is a single
//!      `embeddings_cache` table keyed by sha256(model_id || content)
//!      → vector blob. Reindex looks up first, embeds only on miss.
//!   2. **Vector index gets slow past 50k rows.** Shard the SQLite
//!      vec index by `kind` so each shard stays under the FLAT-vs-
//!      HNSW threshold. The shard policy lives here so the index
//!      module and the migration both read from the same source.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::memory::types::MemoryKind;

pub const HNSW_THRESHOLD_ROWS: usize = 50_000;
pub const EMBEDDING_CACHE_TABLE: &str = "embeddings_cache";

/// Compute the cache key for a `(model_id, content)` pair. Stable
/// across processes — sha256 of the concatenation with a separator
/// so "abc" + "def" and "ab" + "cdef" hash to different keys.
pub fn cache_key(model_id: &str, content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(model_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(content.as_bytes());
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(nibble(b >> 4));
        out.push(nibble(b & 0x0f));
    }
    out
}

fn nibble(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        _ => (b'a' + (n - 10)) as char,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheRow {
    pub key: String,
    pub model_id: String,
    pub dimensions: usize,
    /// Length of the embedding in bytes (4 × dimensions for f32 vectors).
    pub bytes: usize,
}

/// SQL the migration runs on a fresh memory database. Idempotent —
/// `IF NOT EXISTS` everywhere.
pub const CREATE_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS embeddings_cache (\n    \
    key TEXT PRIMARY KEY,\n    \
    model_id TEXT NOT NULL,\n    \
    dimensions INTEGER NOT NULL,\n    \
    vector BLOB NOT NULL,\n    \
    created_at TEXT NOT NULL DEFAULT (datetime('now'))\n\
);";

/// Index sharding policy. Past `HNSW_THRESHOLD_ROWS` total memories
/// of a kind, the index module switches that shard to an HNSW
/// table; below it stays FLAT for cheaper build cost.
pub fn shard_should_hnsw(rows_in_kind: usize) -> bool {
    rows_in_kind > HNSW_THRESHOLD_ROWS
}

/// Each memory kind gets its own vec table named `memory_vec_<kind>`.
/// Returns the canonical table name the index + the migration speak.
pub fn shard_table_name(kind: MemoryKind) -> String {
    match kind {
        MemoryKind::Claim => "memory_vec_claim".into(),
        MemoryKind::Pref => "memory_vec_pref".into(),
        MemoryKind::Person => "memory_vec_person".into(),
        MemoryKind::Observe => "memory_vec_observe".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_is_deterministic() {
        let a = cache_key("openai-text-embedding-3-large", "hello world");
        let b = cache_key("openai-text-embedding-3-large", "hello world");
        assert_eq!(a, b);
    }

    #[test]
    fn cache_key_differs_per_model() {
        let a = cache_key("openai-text-embedding-3-large", "hello");
        let b = cache_key("bge-small-en-v1.5", "hello");
        assert_ne!(a, b);
    }

    #[test]
    fn cache_key_differs_per_content() {
        let a = cache_key("model", "first");
        let b = cache_key("model", "second");
        assert_ne!(a, b);
    }

    #[test]
    fn cache_key_handles_separator_correctly() {
        let a = cache_key("ab", "cdef");
        let b = cache_key("abc", "def");
        assert_ne!(a, b, "separator must distinguish prefix shifts");
    }

    #[test]
    fn shard_should_hnsw_crosses_at_threshold() {
        assert!(!shard_should_hnsw(0));
        assert!(!shard_should_hnsw(HNSW_THRESHOLD_ROWS));
        assert!(shard_should_hnsw(HNSW_THRESHOLD_ROWS + 1));
    }

    #[test]
    fn shard_table_names_are_unique_per_kind() {
        let mut names: Vec<String> = [
            MemoryKind::Claim,
            MemoryKind::Pref,
            MemoryKind::Person,
            MemoryKind::Observe,
        ]
        .iter()
        .map(|k| shard_table_name(*k))
        .collect();
        names.sort();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before);
    }

    #[test]
    fn cache_key_returns_64_hex_chars() {
        let k = cache_key("m", "c");
        assert_eq!(k.len(), 64);
        assert!(k.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
