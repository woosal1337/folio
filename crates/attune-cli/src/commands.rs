//! Per-subcommand handlers for `attune-cli`.
//!
//! Each child module owns one CLI verb. `main.rs` matches on the
//! parsed [`crate::cli::Command`] and forwards to the matching `run_*`
//! function here.

pub mod devices;
pub mod record;
pub mod transcribe;

#[cfg(target_os = "macos")]
pub mod vpio_smoke;
