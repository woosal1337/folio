pub mod dream_loop;
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
