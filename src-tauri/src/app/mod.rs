//! Application-wide concerns: the process-state singleton and platform
//! glue (macOS Dock icon).

pub mod dock_icon;
pub mod state;

pub use state::AppState;
