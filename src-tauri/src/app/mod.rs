//! Application-wide concerns: the process-state singleton and platform
//! glue (macOS Dock icon).

pub mod dock_icon;
pub mod meeting_watcher;
pub mod share_sheet;
pub mod state;
pub mod tray;
pub mod vibrancy;

pub use state::AppState;
