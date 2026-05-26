//! Local-first memory layer.
//!
//! Markdown files are the source of truth (Camp 2 / context-substrate
//! pattern: portable, diffable, harness-independent). SQLite +
//! FTS5 + sqlite-vec is the disposable derived index. The store
//! enforces two-phase write so a crash mid-update never leaves the
//! truth file out of sync: the .md file lands first, the index
//! second, and a `rebuild_index` call heals the index from the
//! files in seconds.
//!
//! Memory pages use the compiled-truth + timeline format from GBrain:
//! frontmatter carries the machine-readable contract; the body shows
//! the current value and a timeline of supersedes. Keyed kinds
//! (Claim/Pref/Person) conflict-resolve via Mem0's ADD/UPDATE/NOOP
//! protocol with Zep-style bi-temporal `valid_until` semantics —
//! superseded memories stay on disk for audit, the index hides them
//! from default queries.
//!
//! The extraction surface (a `remember` tool exposed to the
//! `extract-memories` agent) lives in the Tauri command layer, not
//! here. This module is intentionally agent-agnostic so a CLI or
//! integration test can drive it without spinning up an LLM.

pub mod embed;
pub mod embedding_cache;
pub mod embedding_provider;
pub mod git_commit;
pub mod index;
pub mod page;
pub mod store;
pub mod types;
pub mod watcher;

pub use embed::EmbeddingClient;
pub use index::{MemoryIndex, EMBEDDING_DIMS};
pub use page::{filename_for, parse_page, path_for, read_dir_pages, render_page, write_page};
pub use store::{CreateOutcome, MemoryStore};
pub use types::{Memory, MemoryKind, MemoryQuery, MemoryUpdate, NewMemory};
