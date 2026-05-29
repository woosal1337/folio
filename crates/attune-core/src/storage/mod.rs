//! On-disk persistence: user settings and recording-session metadata.
//!
//! Audio capture and transcription are stateless from the library's
//! perspective; this module owns everything that needs to survive between
//! process restarts. Settings live in the platform's standard config
//! location (`~/Library/Application Support/Attune/settings.json` on
//! macOS). Recording metadata is derived from the session directories the
//! capture pipeline creates.

pub mod atomic_write;
pub mod decisions;
pub mod digest;
pub mod egress_log;
pub mod folders;
pub mod fs_io;
pub mod git_sync;
pub mod note_export;
pub mod retention;
pub mod search;
pub mod session;
pub mod settings;
pub mod share_bundle;
pub mod showcase;
pub mod snapshot;
pub mod spotlight;
pub mod tasks;
pub mod vault_layout;

pub use session::{scan_recordings, RecordingSummary};
pub use settings::{Settings, SettingsStore};
pub use tasks::{NewTask, Task, TaskStatus, TaskStore, TaskUpdate};
