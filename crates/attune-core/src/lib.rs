//! Attune core library.
//!
//! Audio capture, transcription, and on-disk persistence for Attune. The
//! crate is intentionally framework-agnostic so it can be embedded in
//! either the Tauri desktop app (`attune-app`), the CLI test harness
//! (`attune-cli`), or — in a future iteration — a Swift app via the
//! [`ffi`] module.
//!
//! ## Module layout
//!
//! - [`audio`] — microphone and system audio capture, resampling, WAV
//!   writing.
//! - [`storage`] — user settings persistence and recording-session
//!   metadata scanning.
//! - [`transcription`] — pluggable speech-to-text backends.
//! - [`llm`] — pluggable chat-completion backends + key storage.
//! - [`ffi`] — UniFFI-friendly surface for non-Rust consumers
//!   (placeholder).
//! - [`error`] — the single public [`AttuneError`] enum.

pub mod audio;
pub mod calendar;
pub mod cloud_guard;
pub mod error;
pub mod evals;
pub mod ffi;
pub mod highlight_reel;
pub mod import;
pub mod llm;
pub mod memory;
pub mod onboarding;
pub mod permissions;
pub mod qos;
pub mod storage;
pub mod transcription;
pub mod webhooks;

pub use error::{AttuneError, Result};
