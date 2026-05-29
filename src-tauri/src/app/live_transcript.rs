//! Live streaming transcript preview (GET-160).
//!
//! Granola streams a live caption into the dock while recording. Attune
//! transcribes the finished WAVs on Stop (the source of truth); this adds
//! a *preview* on top without touching the capture hot path.
//!
//! Approach: a background thread reads the raw PCM tail of the
//! in-progress `mic.wav` every few seconds, runs the local Whisper model
//! over that rolling window, and emits a `live-transcript` event the
//! record dock renders. Reading the file's tail (rather than tapping the
//! audio callback) keeps capture untouched; the few KB the WAV writer
//! has buffered but not flushed are at most a fraction of a second
//! behind, which is fine for a preview. Local-only — safe under
//! `privacy_mode`; the OpenAI path is skipped (no per-chunk cloud calls).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use attune_core::transcription::{LocalWhisperTranscriber, Transcriber};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};
use tracing::debug;

/// Tauri event name carrying a [`LiveTranscript`] preview.
pub const LIVE_TRANSCRIPT_EVENT: &str = "live-transcript";

/// How often the preview re-runs Whisper over the rolling window.
const POLL_INTERVAL: Duration = Duration::from_secs(3);
/// How many seconds of the most recent audio to feed each pass.
const WINDOW_SECS: usize = 12;
/// Canonical PCM WAV header size hound writes (RIFF + fmt + data headers).
const WAV_HEADER_BYTES: u64 = 44;

/// Preview payload emitted to the record dock.
#[derive(Debug, Clone, Serialize)]
pub struct LiveTranscript {
    /// The capturing note's session dir, so the dock only shows the
    /// preview for the note it belongs to.
    pub session_dir: String,
    /// The rolling-window transcript text (a preview, not final).
    pub text: String,
}

/// Spawn the live-preview loop on a background thread. It runs until
/// `stop` flips to `true` (set by stop/pause). `model_path` is the
/// resolved local Whisper model; `language` is the optional hint.
pub fn spawn<R: Runtime>(
    app: AppHandle<R>,
    session_dir: PathBuf,
    model_path: PathBuf,
    language: Option<String>,
    stop: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let transcriber = LocalWhisperTranscriber::new(model_path);
        let mic_path = session_dir.join("mic.wav");
        let session_id = session_dir.to_string_lossy().into_owned();
        let mut last_emitted = String::new();

        while !stop.load(Ordering::Relaxed) {
            // Sleep first so the file has some audio before the first pass.
            for _ in 0..POLL_INTERVAL.as_secs() {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                std::thread::sleep(Duration::from_secs(1));
            }

            let Some((rate, samples)) = read_wav_tail_mono(&mic_path, WINDOW_SECS) else {
                continue;
            };
            if samples.len() < rate as usize {
                // Less than ~1s of audio — nothing worth previewing yet.
                continue;
            }

            // Whisper wants a file; write the window to a temp WAV at the
            // mic's native rate (the transcriber resamples to 16k itself).
            let tmp = std::env::temp_dir().join(format!("attune-live-{}.wav", std::process::id()));
            if write_mono_wav(&tmp, rate, &samples).is_err() {
                continue;
            }
            let text = match transcriber.transcribe(&tmp, language.as_deref()) {
                Ok(t) => t.full_text().trim().to_string(),
                Err(e) => {
                    debug!(error = %e, "live transcript pass failed");
                    continue;
                }
            };
            let _ = std::fs::remove_file(&tmp);

            if text.is_empty() || text == last_emitted {
                continue;
            }
            last_emitted = text.clone();
            let _ = app.emit(
                LIVE_TRANSCRIPT_EVENT,
                LiveTranscript {
                    session_dir: session_id.clone(),
                    text,
                },
            );
        }
    });
}

/// Read the last `window_secs` of a (possibly still-being-written) PCM
/// WAV as mono `i16` samples, returning `(sample_rate, samples)`.
/// Returns `None` when the file is missing, too small, or not the
/// 16-bit PCM canonical layout the capture writer produces.
///
/// Pure + testable: parses the header for rate/channels/bits, then reads
/// the raw data tail directly (no hound reader, which needs a finalized
/// header that an in-progress file doesn't have yet).
pub fn read_wav_tail_mono(path: &Path, window_secs: usize) -> Option<(u32, Vec<i16>)> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < WAV_HEADER_BYTES as usize {
        return None;
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    // Canonical hound PCM layout: fmt subchunk at offset 12.
    let channels = u16::from_le_bytes([bytes[22], bytes[23]]).max(1);
    let rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
    let bits = u16::from_le_bytes([bytes[34], bytes[35]]);
    if rate == 0 || bits != 16 {
        return None;
    }

    let data = &bytes[WAV_HEADER_BYTES as usize..];
    let frame_bytes = (channels as usize) * 2; // 16-bit
    let total_frames = data.len() / frame_bytes;
    let window_frames = window_secs * rate as usize;
    let take_frames = window_frames.min(total_frames);
    let start_frame = total_frames - take_frames;

    let mut out = Vec::with_capacity(take_frames);
    for f in start_frame..total_frames {
        let base = f * frame_bytes;
        // Downmix to mono by averaging channels.
        let mut acc: i32 = 0;
        for c in 0..channels as usize {
            let o = base + c * 2;
            acc += i16::from_le_bytes([data[o], data[o + 1]]) as i32;
        }
        out.push((acc / channels as i32) as i16);
    }
    Some((rate, out))
}

/// Write mono `i16` samples to a canonical PCM WAV at `rate`.
fn write_mono_wav(path: &Path, rate: u32, samples: &[i16]) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for s in samples {
        writer.write_sample(*s).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_wav(path: &Path, rate: u32, channels: u16, n: usize) {
        let spec = hound::WavSpec {
            channels,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..n {
            for _ in 0..channels {
                w.write_sample((i % 100) as i16).unwrap();
            }
        }
        w.finalize().unwrap();
    }

    #[test]
    fn reads_tail_of_mono_wav() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("mic.wav");
        // 5s of 16k mono.
        write_test_wav(&path, 16_000, 1, 5 * 16_000);
        let (rate, samples) = read_wav_tail_mono(&path, 2).unwrap();
        assert_eq!(rate, 16_000);
        // Window caps at 2s.
        assert_eq!(samples.len(), 2 * 16_000);
    }

    #[test]
    fn caps_window_to_available_audio() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("mic.wav");
        // Only 1s available, ask for 12s.
        write_test_wav(&path, 16_000, 1, 16_000);
        let (_, samples) = read_wav_tail_mono(&path, 12).unwrap();
        assert_eq!(samples.len(), 16_000);
    }

    #[test]
    fn downmixes_stereo_to_mono() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("sys.wav");
        write_test_wav(&path, 48_000, 2, 48_000);
        let (rate, samples) = read_wav_tail_mono(&path, 1).unwrap();
        assert_eq!(rate, 48_000);
        assert_eq!(samples.len(), 48_000); // frames, not interleaved samples
    }

    #[test]
    fn rejects_non_wav() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("not.wav");
        std::fs::write(&path, b"this is not a wav file at all......").unwrap();
        assert!(read_wav_tail_mono(&path, 5).is_none());
    }

    #[test]
    fn roundtrips_through_write_mono_wav() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("out.wav");
        let samples: Vec<i16> = (0..1000).map(|i| (i % 50) as i16).collect();
        write_mono_wav(&path, 16_000, &samples).unwrap();
        let (rate, read) = read_wav_tail_mono(&path, 1).unwrap();
        assert_eq!(rate, 16_000);
        assert_eq!(read.len(), samples.len());
        assert_eq!(read, samples);
    }
}
