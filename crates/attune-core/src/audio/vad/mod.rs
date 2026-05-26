//! Voice Activity Detection — speech-presence detectors used by the
//! pre-transcription pipeline.
//!
//! The current default is [`silero`], a port of cobanov/autocut's
//! Silero V5 wrapper. The previous per-window RMS gate still lives in
//! `transcription::vad` as a runtime fallback for the rare cases the
//! Silero ONNX runtime fails to initialise (e.g. extremely stripped
//! sandbox where the model can't be loaded).
//!
//! Reference material:
//! - 80-source synthesis: `obsidian.md/wiki/voice-activity-detection.md`
//! - Migration plan: `obsidian.md/projects/attune/plan/silero-vad-migration.md`

pub mod silero;
