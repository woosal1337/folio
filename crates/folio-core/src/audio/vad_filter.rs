use std::path::{Path, PathBuf};

use hound::{SampleFormat, WavReader, WavWriter};
use serde::{Deserialize, Serialize};

use crate::audio::resampler::StreamingResampler;
use crate::audio::vad::silero;
use crate::error::{FolioError, Result};
use crate::transcription::vad::{active_ranges_with, ActiveRange};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VadEngine {
    #[default]
    Silero,
    Rms,
}

const PAD_MS: u64 = 250;

const SILENCE_PAD_MS: u64 = 300;

const RMS_FLOOR: f32 = 0.0056;

const MIN_GAP_SECS: f32 = 2.0;

const VAD_SAMPLE_RATE: u32 = 16_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadSidecar {
    pub sample_rate: u32,

    pub original_samples: u64,

    pub kept_samples: u64,

    pub ranges: Vec<VadRangeMapping>,

    pub silence_stripped_seconds: f64,

    pub active_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadRangeMapping {
    pub original_start_sample: u64,
    pub original_end_sample: u64,
    pub cut_start_sample: u64,
    pub cut_end_sample: u64,
}

#[derive(Debug, Clone)]
pub struct VadFilterOutcome {
    pub speech_wav_path: PathBuf,

    pub sidecar_path: PathBuf,

    pub sidecar: VadSidecar,
}

pub fn apply_vad_to_wav_with(input_wav: &Path, engine: VadEngine) -> Result<VadFilterOutcome> {
    let stem = input_wav
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            FolioError::Transcription(format!(
                "vad: input path {} has no usable stem",
                input_wav.display()
            ))
        })?
        .to_string();
    apply_vad_to_wav_with_stem(input_wav, engine, &stem)
}

pub fn apply_vad_to_wav_with_stem(
    input_wav: &Path,
    engine: VadEngine,
    out_stem: &str,
) -> Result<VadFilterOutcome> {
    let reader = WavReader::open(input_wav).map_err(|e| {
        FolioError::Transcription(format!("vad: could not open {}: {e}", input_wav.display()))
    })?;
    let spec = reader.spec();
    let source_samples = read_interleaved_f32(reader)?;
    let original_frame_count = (source_samples.len() / spec.channels.max(1) as usize) as u64;

    let mono16k = to_mono_16k(&source_samples, spec.channels, spec.sample_rate)?;

    let mono_len = mono16k.len();
    let ranges: Vec<ActiveRange> = match engine {
        VadEngine::Silero => match silero::detect(&mono16k, silero::SileroParams::default()) {
            Ok(segs) => segs
                .into_iter()
                .map(|s| ActiveRange {
                    start: (s.start_seconds * VAD_SAMPLE_RATE as f64) as usize,
                    end: ((s.end_seconds * VAD_SAMPLE_RATE as f64) as usize).min(mono_len),
                })
                .collect(),
            Err(e) => {
                tracing::warn!(error = %e, "silero detect failed; falling back to RMS gate");
                active_ranges_with(
                    &mono16k,
                    VAD_SAMPLE_RATE,
                    VAD_SAMPLE_RATE as usize * 30,
                    RMS_FLOOR,
                    MIN_GAP_SECS,
                )
            }
        },
        VadEngine::Rms => active_ranges_with(
            &mono16k,
            VAD_SAMPLE_RATE,
            VAD_SAMPLE_RATE as usize * 30,
            RMS_FLOOR,
            MIN_GAP_SECS,
        ),
    };

    let padded = pad_and_merge_ranges(&ranges, mono_len, VAD_SAMPLE_RATE, PAD_MS);

    let scale = spec.sample_rate as f64 / VAD_SAMPLE_RATE as f64;
    let source_ranges: Vec<ActiveRange> = padded
        .iter()
        .map(|r| {
            let start = ((r.start as f64) * scale).floor() as usize;
            let end = ((r.end as f64) * scale).ceil() as usize;
            ActiveRange {
                start: start.min(original_frame_count as usize),
                end: end.min(original_frame_count as usize),
            }
        })
        .collect();

    let parent = input_wav.parent().unwrap_or_else(|| Path::new("."));
    let speech_path = parent.join(format!("{out_stem}.speech.wav"));
    let sidecar_path = parent.join(format!("{out_stem}.vad.json"));

    let speech_silence_frames =
        ((spec.sample_rate as u64 * SILENCE_PAD_MS) as f64 / 1000.0).round() as u64;
    let mut writer = WavWriter::create(&speech_path, spec).map_err(|e| {
        FolioError::Transcription(format!(
            "vad: could not create {}: {e}",
            speech_path.display()
        ))
    })?;
    let channels = spec.channels as usize;
    let mut cut_cursor: u64 = 0;
    let mut mappings: Vec<VadRangeMapping> = Vec::with_capacity(source_ranges.len());
    let pad_sample = make_pad_sample(spec.sample_format);

    for (i, range) in source_ranges.iter().enumerate() {
        if i > 0 && speech_silence_frames > 0 {
            for _ in 0..speech_silence_frames {
                for _ in 0..channels {
                    write_sample(&mut writer, spec.sample_format, pad_sample)?;
                }
            }
            cut_cursor += speech_silence_frames;
        }

        let frame_start = range.start as u64;
        let frame_end = range.end as u64;
        let cut_start = cut_cursor;

        for frame_idx in frame_start..frame_end {
            for ch in 0..channels {
                let idx = (frame_idx as usize) * channels + ch;
                if idx < source_samples.len() {
                    let s = source_samples[idx];
                    write_sample_f32(&mut writer, spec.sample_format, s)?;
                }
            }
        }
        let frames_written = frame_end.saturating_sub(frame_start);
        cut_cursor += frames_written;

        mappings.push(VadRangeMapping {
            original_start_sample: frame_start,
            original_end_sample: frame_end,
            cut_start_sample: cut_start,
            cut_end_sample: cut_cursor,
        });
    }
    writer.finalize().map_err(|e| {
        FolioError::Transcription(format!(
            "vad: finalising {} failed: {e}",
            speech_path.display()
        ))
    })?;

    let original_seconds = original_frame_count as f64 / spec.sample_rate as f64;
    let kept_seconds = cut_cursor as f64 / spec.sample_rate as f64;

    let silence_stripped_seconds = (original_seconds - kept_seconds).max(0.0);
    let active_ratio = if original_frame_count == 0 {
        0.0
    } else {
        let active_frames: u64 = mappings
            .iter()
            .map(|m| m.original_end_sample - m.original_start_sample)
            .sum();
        active_frames as f64 / original_frame_count as f64
    };

    let sidecar = VadSidecar {
        sample_rate: spec.sample_rate,
        original_samples: original_frame_count,
        kept_samples: cut_cursor,
        ranges: mappings,
        silence_stripped_seconds,
        active_ratio,
    };
    let sidecar_json = serde_json::to_vec_pretty(&sidecar)
        .map_err(|e| FolioError::Transcription(format!("vad: serialising sidecar failed: {e}")))?;
    std::fs::write(&sidecar_path, sidecar_json).map_err(|e| {
        FolioError::Transcription(format!(
            "vad: writing {} failed: {e}",
            sidecar_path.display()
        ))
    })?;

    Ok(VadFilterOutcome {
        speech_wav_path: speech_path,
        sidecar_path,
        sidecar,
    })
}

pub fn remap_cut_seconds_to_original(sidecar: &VadSidecar, cut_seconds: f64) -> f64 {
    if sidecar.sample_rate == 0 {
        return cut_seconds;
    }
    let cut_sample = (cut_seconds * sidecar.sample_rate as f64).round() as u64;
    for range in &sidecar.ranges {
        if cut_sample >= range.cut_start_sample && cut_sample <= range.cut_end_sample {
            let offset = cut_sample - range.cut_start_sample;
            let original = range.original_start_sample + offset;
            return original as f64 / sidecar.sample_rate as f64;
        }
    }

    if let Some(next) = sidecar
        .ranges
        .iter()
        .find(|r| r.cut_start_sample > cut_sample)
    {
        return next.original_start_sample as f64 / sidecar.sample_rate as f64;
    }

    sidecar.original_samples as f64 / sidecar.sample_rate as f64
}

fn pad_and_merge_ranges(
    ranges: &[ActiveRange],
    total_samples: usize,
    sample_rate: u32,
    pad_ms: u64,
) -> Vec<ActiveRange> {
    if ranges.is_empty() {
        return Vec::new();
    }
    let pad_samples = ((sample_rate as u64 * pad_ms) / 1000) as usize;
    let mut out: Vec<ActiveRange> = Vec::with_capacity(ranges.len());
    for range in ranges {
        let start = range.start.saturating_sub(pad_samples);
        let end = (range.end + pad_samples).min(total_samples);
        match out.last_mut() {
            Some(last) if start <= last.end => {
                last.end = last.end.max(end);
            }
            _ => out.push(ActiveRange { start, end }),
        }
    }
    out
}

fn to_mono_16k(samples: &[f32], channels: u16, sample_rate: u32) -> Result<Vec<f32>> {
    if channels == 1 && sample_rate == VAD_SAMPLE_RATE {
        return Ok(samples.to_vec());
    }
    let mut resampler = StreamingResampler::new(sample_rate, channels, VAD_SAMPLE_RATE)?;
    let mut out = resampler.process(samples)?;
    out.extend(resampler.flush()?);
    Ok(out)
}

fn read_interleaved_f32<R: std::io::Read>(reader: WavReader<R>) -> Result<Vec<f32>> {
    let spec = reader.spec();
    let mut out: Vec<f32> = Vec::with_capacity(reader.len() as usize);
    match spec.sample_format {
        SampleFormat::Float => {
            for s in reader.into_samples::<f32>() {
                out.push(s.map_err(|e| {
                    FolioError::Transcription(format!("vad: wav read failed: {e}"))
                })?);
            }
        }
        SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            if !(8..=32).contains(&bits) {
                return Err(FolioError::Transcription(format!(
                    "vad: unsupported PCM bit depth: {bits}"
                )));
            }
            let max = (1i64 << (bits - 1)) as f32;
            for s in reader.into_samples::<i32>() {
                let raw =
                    s.map_err(|e| FolioError::Transcription(format!("vad: wav read failed: {e}")))?;
                out.push(raw as f32 / max);
            }
        }
    }
    Ok(out)
}

fn make_pad_sample(_format: SampleFormat) -> f32 {
    0.0
}

fn write_sample_f32<W: std::io::Write + std::io::Seek>(
    writer: &mut WavWriter<W>,
    format: SampleFormat,
    value: f32,
) -> Result<()> {
    match format {
        SampleFormat::Float => writer
            .write_sample(value)
            .map_err(|e| FolioError::Transcription(format!("vad: wav write failed: {e}"))),
        SampleFormat::Int => {
            let spec = writer.spec();
            let bits = spec.bits_per_sample.max(1);
            let max = (1i64 << (bits - 1)) as f32;
            let clamped = value.clamp(-1.0, 1.0);

            let lo = -(1i64 << (bits - 1)) as i32;
            let hi = ((1i64 << (bits - 1)) - 1) as i32;
            let int_sample = ((clamped * max).round() as i32).clamp(lo, hi);
            writer
                .write_sample(int_sample)
                .map_err(|e| FolioError::Transcription(format!("vad: wav write failed: {e}")))
        }
    }
}

fn write_sample<W: std::io::Write + std::io::Seek>(
    writer: &mut WavWriter<W>,
    format: SampleFormat,
    value: f32,
) -> Result<()> {
    write_sample_f32(writer, format, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::{SampleFormat as HSF, WavSpec};
    use std::f32::consts::PI;
    use tempfile::TempDir;

    fn write_test_wav(path: &Path, sample_rate: u32, channels: u16, samples: &[f32]) {
        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: HSF::Int,
        };
        let mut writer = WavWriter::create(path, spec).unwrap();
        for s in samples {
            let int_sample = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(int_sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    fn loud_sine(samples: usize, freq_hz: u32, sample_rate: u32) -> Vec<f32> {
        (0..samples)
            .map(|i| (2.0 * PI * freq_hz as f32 * i as f32 / sample_rate as f32).sin() * 0.5)
            .collect()
    }

    #[test]
    fn pure_silence_produces_empty_speech_wav_and_zero_ranges() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mic.wav");
        write_test_wav(&path, 16_000, 1, &vec![0.0_f32; 16_000 * 90]);

        for engine in [VadEngine::Silero, VadEngine::Rms] {
            let outcome = apply_vad_to_wav_with(&path, engine).unwrap();
            assert_eq!(outcome.sidecar.ranges.len(), 0, "engine = {engine:?}");
            assert_eq!(outcome.sidecar.kept_samples, 0, "engine = {engine:?}");
            assert!(
                outcome.sidecar.silence_stripped_seconds >= 89.0,
                "engine = {engine:?}"
            );
            assert_eq!(outcome.sidecar.active_ratio, 0.0, "engine = {engine:?}");
        }
    }

    #[test]
    fn rms_keeps_a_pure_loud_signal_in_full() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mic.wav");
        write_test_wav(&path, 16_000, 1, &loud_sine(16_000 * 30, 440, 16_000));

        let outcome = apply_vad_to_wav_with(&path, VadEngine::Rms).unwrap();
        assert_eq!(outcome.sidecar.ranges.len(), 1);

        assert!(outcome.sidecar.kept_samples >= 16_000 * 30);
        assert!(outcome.sidecar.active_ratio > 0.99);
    }

    #[test]
    fn rms_collapses_silence_between_loud_islands() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mic.wav");
        let mut buf = loud_sine(16_000 * 30, 440, 16_000);
        buf.extend(std::iter::repeat_n(0.0_f32, 16_000 * 60));
        buf.extend(loud_sine(16_000 * 30, 440, 16_000));
        write_test_wav(&path, 16_000, 1, &buf);

        let outcome = apply_vad_to_wav_with(&path, VadEngine::Rms).unwrap();
        assert_eq!(outcome.sidecar.ranges.len(), 2);

        assert!(outcome.sidecar.silence_stripped_seconds > 30.0);
    }

    #[test]
    #[ignore = "requires FOLIO_VAD_FIXTURE=<session_dir>"]
    fn compare_engines_on_recording() {
        let fixture = match std::env::var("FOLIO_VAD_FIXTURE") {
            Ok(s) => std::path::PathBuf::from(s),
            Err(_) => panic!(
                "set FOLIO_VAD_FIXTURE to a session directory containing mic.wav / system.wav"
            ),
        };
        println!("\nfixture: {}", fixture.display());
        for ch in ["mic", "system"] {
            let path = fixture.join(format!("{ch}.wav"));
            if !path.is_file() {
                continue;
            }
            for engine in [VadEngine::Silero, VadEngine::Rms] {
                let t0 = std::time::Instant::now();
                let outcome = apply_vad_to_wav_with(&path, engine).unwrap();
                let dt = t0.elapsed();
                let s = &outcome.sidecar;
                println!(
                    "{ch:>7}  engine={engine:?}  ranges={:<3}  active={:.3}  kept={:>6.1}s  stripped={:>6.1}s  wall={:?}",
                    s.ranges.len(),
                    s.active_ratio,
                    s.kept_samples as f64 / s.sample_rate as f64,
                    s.silence_stripped_seconds,
                    dt,
                );
            }
        }
    }

    #[test]
    fn silero_rejects_pure_sine_as_non_speech() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mic.wav");
        write_test_wav(&path, 16_000, 1, &loud_sine(16_000 * 30, 440, 16_000));

        let outcome = apply_vad_to_wav_with(&path, VadEngine::Silero).unwrap();
        assert_eq!(outcome.sidecar.ranges.len(), 0);
        assert_eq!(outcome.sidecar.kept_samples, 0);
        assert!(outcome.sidecar.silence_stripped_seconds >= 29.0);
    }

    #[test]
    fn remap_lands_on_original_timeline_for_in_range_samples() {
        let sidecar = VadSidecar {
            sample_rate: 16_000,
            original_samples: 16_000 * 90,
            kept_samples: 16_000 * 60,
            ranges: vec![
                VadRangeMapping {
                    original_start_sample: 0,
                    original_end_sample: 16_000 * 30,
                    cut_start_sample: 0,
                    cut_end_sample: 16_000 * 30,
                },
                VadRangeMapping {
                    original_start_sample: 16_000 * 60,
                    original_end_sample: 16_000 * 90,
                    cut_start_sample: 16_000 * 30,
                    cut_end_sample: 16_000 * 60,
                },
            ],
            silence_stripped_seconds: 30.0,
            active_ratio: 60.0 / 90.0,
        };

        assert!((remap_cut_seconds_to_original(&sidecar, 15.0) - 15.0).abs() < 0.01);

        assert!((remap_cut_seconds_to_original(&sidecar, 45.0) - 75.0).abs() < 0.01);
    }
}
