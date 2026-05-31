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

use attune_core::audio::enhancement::{self, EnhancementConfig};
use attune_core::audio::vad_filter::{apply_vad_to_wav_with_stem, VadEngine, VadSidecar};

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
    let (output_dir, enh_enabled, enh_cfg) = {
        let s = state.settings.lock();
        (
            s.output_dir.clone(),
            s.system_audio_enhancement.enabled,
            EnhancementConfig {
                atten_lim_db: s.system_audio_enhancement.atten_lim_db,
            },
        )
    };

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

            // System-audio speech enhancement (GET-188) runs as a
            // pre-pass on the raw `system.wav`, writing
            // `system.enhanced.wav` that VAD then consumes. The raw
            // recording is preserved. Only the system channel is
            // enhanced — the mic channel already goes through Voice
            // Processing IO. Any failure falls back to the raw audio.
            let vad_input = if *ch == "system" && enh_enabled {
                let enhanced = work_dir.join("system.enhanced.wav");
                match enhancement::enhance_wav_file(&path, &enhanced, &enh_cfg) {
                    Ok(stats) => {
                        info!(
                            channel = ch,
                            rtf = stats.rtf(),
                            input_rms = stats.input_rms,
                            output_rms = stats.output_rms,
                            audio_secs = stats.audio_secs,
                            "enhancement: system channel enhanced"
                        );
                        enhanced
                    }
                    Err(e) => {
                        warn!(channel = ch, error = %e, "enhancement failed; using raw system audio");
                        path.clone()
                    }
                }
            } else {
                path.clone()
            };

            // Pin the output stem to the channel name so the enhanced
            // input still yields the canonical `<ch>.speech.wav` +
            // `<ch>.vad.json` the transcription step looks for.
            match apply_vad_to_wav_with_stem(&vad_input, VadEngine::default(), ch) {
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
