#[cfg(target_os = "macos")]
pub mod audio_input_watcher;
pub mod dock_icon;
pub mod event_kit;
pub mod live_transcript;
pub mod meeting_watcher;
pub mod share_sheet;
pub mod state;
pub mod sync_scheduler;
pub mod tray;
pub mod vibrancy;
pub mod window_aside;

pub use state::AppState;
