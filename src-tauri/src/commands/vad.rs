//! Pre-transcription VAD pipeline.
//!
//! `run_vad` reads `<session_dir>/mic.wav` and
//! `<session_dir>/system.wav` (whichever exist) and writes a
//! `<channel>.speech.wav` + `<channel>.vad.json` next to each via
//! `attune_core::audio::vad_filter`. The transcription command picks
//! these up automatically the next time it runs.
//!
//! The frontend queues this as its own job kind so the user sees a
//! "VAD…" pill on the job strip before the "Transcribing…" pill
//! follows. The two-step queue (VAD then transcribe) is intentional:
//! cloud-Whisper users see a smaller upload, local-Whisper users see
//! a shorter inference, and both avoid the silence-hallucination
//! failure mode that plagued the
//! 2026-05-26-11-47-54 mic recording where Whisper looped
//! "I'm going to ask you to take your own distance from there." 14
//! times over 60 seconds of silence on the mic track.

use std::path::PathBuf;

use serde::Serialize;
use tauri::State;
use tracing::{info, warn};

use attune_core::audio::vad_filter::{apply_vad_to_wav, VadSidecar};

use crate::app::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct ChannelVadResult {
    pub channel: String,
    pub speech_wav_path: PathBuf,
    pub sidecar_path: PathBuf,
    pub sidecar: VadSidecar,
}

#[derive(Debug, Clone, Serialize)]
pub struct VadRunResult {
    pub session_dir: PathBuf,
    pub channels: Vec<ChannelVadResult>,
    /// Channels we wanted to process but couldn't read. Non-fatal —
    /// the transcription step will fall back to the raw WAV for any
    /// channel that fails here.
    pub channel_errors: Vec<String>,
}

/// Run the VAD pre-pass on every audio channel in `session_dir`.
///
/// Idempotent: running twice over the same session overwrites the
/// previous `<channel>.speech.wav` + `<channel>.vad.json`. Safe to
/// call from a retry button or a re-process loop without leaking
/// state.
#[tauri::command]
pub async fn run_vad(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<VadRunResult, String> {
    let output_dir = state.settings.lock().output_dir.clone();

    // Canonicalise under the configured output dir so we don't let a
    // malicious sessionDir argument from the frontend escape the
    // recordings root.
    let canonical = tauri::async_runtime::spawn_blocking(move || {
        attune_core::paths::canonicalize_under(&output_dir, &session_dir).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("canonicalize task panicked: {e}"))??;

    let work_dir = canonical.clone();
    let outcome = tauri::async_runtime::spawn_blocking(move || -> VadRunResult {
        let mut channels = Vec::new();
        let mut channel_errors = Vec::new();
        for ch in &["mic", "system"] {
            let path = work_dir.join(format!("{ch}.wav"));
            if !path.is_file() {
                continue;
            }
            match apply_vad_to_wav(&path) {
                Ok(o) => {
                    info!(
                        channel = ch,
                        original_samples = o.sidecar.original_samples,
                        kept_samples = o.sidecar.kept_samples,
                        active_ratio = o.sidecar.active_ratio,
                        stripped_secs = o.sidecar.silence_stripped_seconds,
                        "vad: channel processed"
                    );
                    channels.push(ChannelVadResult {
                        channel: (*ch).to_string(),
                        speech_wav_path: o.speech_wav_path,
                        sidecar_path: o.sidecar_path,
                        sidecar: o.sidecar,
                    });
                }
                Err(e) => {
                    warn!(channel = ch, error = %e, "vad: channel failed");
                    channel_errors.push(format!("{ch}: {e}"));
                }
            }
        }
        VadRunResult {
            session_dir: work_dir,
            channels,
            channel_errors,
        }
    })
    .await
    .map_err(|e| format!("vad task panicked: {e}"))?;

    if outcome.channels.is_empty() && !outcome.channel_errors.is_empty() {
        return Err(format!(
            "vad: every channel failed: {}",
            outcome.channel_errors.join("; ")
        ));
    }
    Ok(outcome)
}
