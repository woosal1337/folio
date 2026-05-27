//! Tauri command handlers, grouped by domain.
//!
//! Each `#[tauri::command]` function is callable from the React frontend
//! via `invoke('command_name', args)`. The wire format and command names
//! are an IPC contract; renames here are breaking changes to the
//! frontend.

pub mod account;
pub mod agents;
pub mod auth;
pub mod calendar;
pub mod devices;
pub mod referrals;
pub mod settings_sync;
pub mod health;
pub mod library;
pub mod llm;
pub mod maintenance;
pub mod memory;
pub mod permissions;
pub mod preferences;
pub mod recording;
pub mod settings;
pub mod tasks;
pub mod transcription;
pub mod tray;
pub mod vad;
pub mod webhooks;
pub mod windows;
