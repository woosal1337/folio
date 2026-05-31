//! Per-subcommand handlers for `attune-cli`.
//!
//! Each child module owns one CLI verb. `main.rs` matches on the
//! parsed [`crate::cli::Command`] and forwards to the matching `run_*`
//! function here.

pub mod devices;
pub mod diarize;
pub mod diarize_transcript;
pub mod enhance_compare;
pub mod memory_search;
pub mod record;
pub mod sessions;
pub mod tasks;
pub mod transcribe;

#[cfg(target_os = "macos")]
pub mod vpio_smoke;
