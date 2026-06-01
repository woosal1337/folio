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
//! Domain primitives:
//!
//! - [`audio`] — microphone + system audio capture, resampling, WAV
//!   writing.
//! - [`transcription`] — pluggable speech-to-text backends (local
//!   whisper.cpp + OpenAI Whisper API + chunking + upload state +
//!   model LRU).
//! - [`llm`] — chat / agent / tool surface, providers, agents, run
//!   cards, skills, templates, marketplace, live in-meeting agent,
//!   keystore.
//! - [`memory`] — Camp-2 memory layer: markdown pages on disk, FTS5
//!   + sqlite-vec index, embedding cache + shard policy, fs-watcher,
//!     git-commit hook, dream-loop consolidation.
//! - [`storage`] — settings + sessions + tasks + decisions + showcase
//!   + share bundles + vault layout + Spotlight sidecars + atomic
//!     writes + retention.
//!
//! Capabilities + cross-cutting concerns:
//!
//! - [`ask_attune`] — cross-library RAG citation contract (#021).
//! - [`calendar`] — EventKit calendar awareness + conference URLs.
//! - [`cloud_guard`] — Privacy Mode airgap toggle (#048).
//! - [`encryption`] — AES-256-GCM + Argon2id at-rest encryption.
//! - [`evals`] — transcription quality eval helpers.
//! - [`highlight_reel`] — decision-dense MP4 picker.
//! - [`import`] — Granola / Otter / Fathom switcher import.
//! - [`live_notes`] — `/action /decision /question` parser.
//! - [`mcp_client`] — `.attune/mcp.toml` MCP client config.
//! - [`mcp_server`] — `attune-mcp` JSON-RPC tool surface.
//! - [`onboarding`] — canned-demo bundle (#002).
//! - [`paths`] — `canonicalize_under` containment helper (§8.1).
//! - [`permissions`] — TCC permission walkthrough types.
//! - [`qos`] — macOS QoS class hints for transcription threads.
//! - [`share_page`] — public share-page payload schema.
//! - [`webhooks`] — outbound HMAC-SHA256 signed webhooks.
//!
//! Plumbing:
//!
//! - [`error`] — the single public [`AttuneError`] enum.
//! - [`ffi`] — UniFFI surface for non-Rust consumers (placeholder).
//!
//! ## Layer rule
//!
//! Per `docs/CODE_STYLE.md` §9.1: `attune-core` MUST NOT import
//! Tauri, browser APIs, or any UI framework. The same code compiles
//! for the CLI and the desktop app.

pub mod ask_attune;
pub mod audio;
pub mod backend;
pub mod briefs;
pub mod calendar;
pub mod cloud_guard;
pub mod diarization;
pub mod encryption;
pub mod error;
pub mod evals;
pub mod ffi;
pub mod highlight_reel;
pub mod import;
pub mod live_notes;
pub mod llm;
pub mod mcp_client;
pub mod mcp_server;
pub mod memory;
pub mod onboarding;
pub mod paths;
pub mod permissions;
pub mod qos;
pub mod recipes;
pub mod share_page;
pub mod speaker_memory;
pub mod storage;
pub mod text;
pub mod transcription;
pub mod webhooks;

pub use error::{AttuneError, Result};
