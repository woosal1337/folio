use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use folio_core::transcription::{LocalWhisperTranscriber, Transcriber};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};
use tracing::debug;

pub const LIVE_TRANSCRIPT_EVENT: &str = "live-transcript";

const POLL_INTERVAL: Duration = Duration::from_secs(3);

const WINDOW_SECS: usize = 12;

const WAV_HEADER_BYTES: u64 = 44;

#[derive(Debug, Clone, Serialize)]
pub struct LiveTranscript {
    pub session_dir: String,

    pub text: String,
}

pub fn spawn<R: Runtime>(
    app: AppHandle<R>,
    session_dir: PathBuf,
    model_path: PathBuf,
    language: Option<String>,
    stop: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("live-transcript".into())
        .spawn(move || {
            let transcriber = LocalWhisperTranscriber::new(model_path);
            let mic_path = session_dir.join("mic.wav");
            let session_id = session_dir.to_string_lossy().into_owned();
            let mut last_emitted = String::new();

            while !stop.load(Ordering::Relaxed) {
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
                    continue;
                }

                let tmp =
                    std::env::temp_dir().join(format!("folio-live-{}.wav", std::process::id()));
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
        })
        .expect("spawn live-transcript thread")
}

pub fn read_wav_tail_mono(path: &Path, window_secs: usize) -> Option<(u32, Vec<i16>)> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < WAV_HEADER_BYTES as usize {
        return None;
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }

    let channels = u16::from_le_bytes([bytes[22], bytes[23]]).max(1);
    let rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
    let bits = u16::from_le_bytes([bytes[34], bytes[35]]);
    if rate == 0 || bits != 16 {
        return None;
    }

    let data = &bytes[WAV_HEADER_BYTES as usize..];
    let frame_bytes = (channels as usize) * 2;
    let total_frames = data.len() / frame_bytes;
    let window_frames = window_secs * rate as usize;
    let take_frames = window_frames.min(total_frames);
    let start_frame = total_frames - take_frames;

    let mut out = Vec::with_capacity(take_frames);
    for f in start_frame..total_frames {
        let base = f * frame_bytes;

        let mut acc: i32 = 0;
        for c in 0..channels as usize {
            let o = base + c * 2;
            acc += i16::from_le_bytes([data[o], data[o + 1]]) as i32;
        }
        out.push((acc / channels as i32) as i16);
    }
    Some((rate, out))
}

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

        write_test_wav(&path, 16_000, 1, 5 * 16_000);
        let (rate, samples) = read_wav_tail_mono(&path, 2).unwrap();
        assert_eq!(rate, 16_000);

        assert_eq!(samples.len(), 2 * 16_000);
    }

    #[test]
    fn caps_window_to_available_audio() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("mic.wav");

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
        assert_eq!(samples.len(), 48_000);
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
