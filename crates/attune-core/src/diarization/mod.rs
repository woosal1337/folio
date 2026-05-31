//! Speaker diarization.
//!
//! Local-only, runs alongside Whisper. Two-stream architecture exploits
//! attune's mic + system-audio asymmetry: the mic stream is the user
//! *by definition* and feeds a rolling anchor embedding; the system
//! stream gets segmented + embedded + clustered into N speakers and
//! gated against the anchor so AEC residual bleed doesn't surface as a
//! phantom "You" cluster. See
//! `obsidian.md/projects/attune/architecture/diarization.md` for the
//! decided architecture and
//! `obsidian.md/projects/attune/plan/diarization-v1-execution.md` for
//! the phased execution plan this module implements.
//!
//! ## Status (P0)
//!
//! - `models` — model registry, on-disk layout, downloader. Wired.
//! - `runtime` — sherpa-onnx wrapper. Stubbed; returns
//!   `DiarizationError::NotImplemented` until P2 / P3 land.
//!
//! ## Layer rule
//!
//! Per `docs/CODE_STYLE.md` §9.1: this module MUST NOT import any
//! Tauri / browser / UI types. Tauri commands that consume it live in
//! `attune-app`.

pub mod models;
pub mod runtime;

pub use models::{
    DiarizationModel, DiarizationModelStatus, DiarizationModelStore, DownloadProgress,
};
pub use runtime::{DiarizationError, DiarizationRuntime};
