//! Tauri command handlers, grouped by domain.
//!
//! Each `#[tauri::command]` function is callable from the React frontend
//! via `invoke('command_name', args)`. The wire format and command names
//! are an IPC contract; renames here are breaking changes to the
//! frontend.

pub mod account;
pub mod agents;
pub mod ask;
pub mod auth;
pub mod calendar;
pub mod chats;
pub mod devices;
pub mod diarization;
pub mod folders;
pub mod health;
pub mod library;
pub mod live_notes;
pub mod llm;
pub mod maintenance;
pub mod meeting;
pub mod memory;
pub mod permissions;
pub mod preferences;
pub mod recipes;
pub mod recording;
pub mod recording_bar;
pub mod referrals;
pub mod settings;
pub mod settings_sync;
pub mod speakers;
pub mod tasks;
pub mod transcription;
pub mod tray;
pub mod vad;
pub mod webhooks;
