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
//! ## Status
//!
//! - `models` — model registry, on-disk layout, downloader. Wired.
//! - `runtime` — sherpa-onnx `OfflineSpeakerDiarization` wrapper
//!   (pyannote segmentation + WeSpeaker embedding + clustering). Real:
//!   [`DiarizationRuntime::diarize_wav`] returns speaker-labelled
//!   segments. The two-stream mic-anchor integration + the
//!   [`crate::speaker_memory`] hookup are the remaining phases.
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
pub use runtime::{DiarizationError, DiarizationOptions, DiarizationRuntime, DiarizedSegment};
