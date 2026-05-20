//! Attune core library.
//!
//! Audio capture, voice activity detection, Whisper transcription, diarization,
//! and vault writes. Designed to be embedded in either a CLI (`attune-cli`) or
//! a Swift app via UniFFI bindings.
//!
//! See the design vault for architecture rationale:
//! `~/Documents/GitHub/obsidian.md/projects/attune/architecture/`

pub mod audio;
pub mod error;

pub use error::{AttuneError, Result};
