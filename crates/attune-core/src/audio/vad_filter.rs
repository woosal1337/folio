//! Pre-transcription Voice Activity Detection.
//!
//! Reads a captured `<channel>.wav` (mic.wav / system.wav), detects the
//! speech-bearing regions via the per-window RMS gate in
//! `transcription::vad`, and writes two artefacts back to disk next to
//! the original:
//!
//!   * `<channel>.speech.wav` — the compacted audio with all detected
//!     silence removed. A short fixed silence (`SILENCE_PAD_MS`) is
//!     inserted between concatenated active ranges so the downstream
//!     ASR sees natural utterance boundaries instead of fused
//!     sentences. The format matches the source WAV (same sample rate,
//!     channel count, bit depth) so any pipeline that handles the
//!     original handles the compacted file unchanged.
//!
//!   * `<channel>.vad.json` — a sidecar that records the mapping from
//!     positions in the compacted file back to positions in the
//!     original recording. Downstream consumers (transcript renderer,
//!     audio scrubber) read this to remap timestamps back onto the
//!     original recording's timeline so the editor's playhead still
//!     lines up with the source audio when the user scrubs.
//!
//! The point of doing VAD here, *before* transcription, is two-fold:
//!
//!   1. Whisper (cloud or local) hallucinates on silence — the
//!      well-known 14× "I'm going to ask you to take your own
//!      distance" loop the 2026-05-26-11-47-54 mic.wav surfaced was
//!      pure silence-hallucination. Stripping silence before the
//!      decoder sees it removes the failure mode entirely.
//!
//!   2. Smaller inputs are cheaper. Cloud Whisper bills per minute of
//!      audio uploaded; local Whisper bills user CPU/Metal time. A
//!      one-hour meeting with 35 minutes of actual speech (typical of
//!      a workshop with long listening stretches) becomes a 35-minute
//!      transcription job instead of a 60-minute one.
//!
//! v2 roadmap finding R09 / GET-VAD-PRE.
//!
//! ## Algorithm
//!
//! The detector is the same per-window RMS gate from
//! `transcription::vad::active_ranges`. RMS is computed over 30s
//! windows at 16 kHz, every window over `RMS_FLOOR` (-45 dBFS) is
//! kept, and gaps under `MIN_GAP_SECS` are bridged so a single
//! sentence isn't split across two ranges.
//!
//! This module wraps that detector with two extra concerns:
//!
//!   * Padding. Every kept range is widened by `PAD_MS` on each side
//!     so the detector doesn't clip the leading consonant or the
//!     trailing fricative of an utterance. The cost is some extra
//!     "almost-silence" reaching the ASR — cheaper than missing the
//!     start of a word.
//!
//!   * Format preservation. The source WAV may be 48 kHz stereo (mic
//!     paths through cpal) or 16 kHz mono (system / VPIO paths). We
//!     keep whatever the source uses; the detector resamples a copy
//!     down to 16 kHz mono for the VAD pass but the WAV we write
//!     preserves the original format so existing players keep
//!     working.

use std::path::{Path, PathBuf};

use hound::{SampleFormat, WavReader, WavWriter};
use serde::{Deserialize, Serialize};

use crate::audio::resampler::StreamingResampler;
use crate::audio::vad::silero;
use crate::error::{AttuneError, Result};
use crate::transcription::vad::{active_ranges_with, ActiveRange};

/// Which detector backs `apply_vad_to_wav`. `Silero` is the default
/// since 2026-05-27 (see `obsidian.md/projects/attune/plan/silero-vad-
/// migration.md`); `Rms` is the legacy per-window energy gate kept as
/// a runtime fallback for sandboxes where the Silero ONNX model
/// fails to load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadEngine {
    Silero,
    Rms,
}

impl Default for VadEngine {
    fn default() -> Self {
        Self::Silero
    }
}

/// Per-side padding applied to every active range before keeping it,
/// so the leading consonant / trailing fricative aren't clipped by the
/// 30s window granularity of the RMS gate.
const PAD_MS: u64 = 250;

/// Silence inserted between two kept ranges in the compacted file.
/// Without this every joined utterance reads as one long sentence to
/// whisper and the decoder loses its natural segmentation cues.
const SILENCE_PAD_MS: u64 = 300;

/// RMS floor in linear amplitude. Same value the detector uses for the
/// 30s window pass — around -45 dBFS. Anything below is treated as
/// non-speech.
const RMS_FLOOR: f32 = 0.0056;

/// Gaps shorter than this between two raw active windows get bridged
/// so a sentence with a natural breath in the middle stays a single
/// range rather than two adjacent ones.
const MIN_GAP_SECS: f32 = 2.0;

/// Internal sample rate the VAD pass runs at. Whisper consumes 16 kHz
/// anyway; the detector inherits the same target.
const VAD_SAMPLE_RATE: u32 = 16_000;

/// Sidecar metadata written next to `<channel>.speech.wav`. Captures
/// enough information to remap a timestamp in the compacted file back
/// to the original recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadSidecar {
    /// Sample rate of both the source WAV and the compacted WAV (they
    /// share the original sample rate — see module docs).
    pub sample_rate: u32,
    /// Total samples in the source WAV (per channel).
    pub original_samples: u64,
    /// Total samples in the compacted WAV (per channel).
    pub kept_samples: u64,
    /// Sample-accurate boundaries of each kept range. `original_*`
    /// fields index into the source WAV; `cut_*` fields index into
    /// the compacted WAV. The padding silence between ranges is NOT
    /// included in any `cut_*` interval — it falls in the gap
    /// `[range[i].cut_end_sample, range[i+1].cut_start_sample]`.
    pub ranges: Vec<VadRangeMapping>,
    /// Wall-clock seconds saved (i.e. silence stripped).
    pub silence_stripped_seconds: f64,
    /// What fraction of the source contained speech (0.0-1.0).
    pub active_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadRangeMapping {
    pub original_start_sample: u64,
    pub original_end_sample: u64,
    pub cut_start_sample: u64,
    pub cut_end_sample: u64,
}

/// Outcome of a VAD pre-pass over a single channel.
#[derive(Debug, Clone)]
pub struct VadFilterOutcome {
    /// Where the compacted speech-only WAV was written.
    pub speech_wav_path: PathBuf,
    /// Where the timestamp-remapping sidecar was written.
    pub sidecar_path: PathBuf,
    /// The same data serialised back to the caller so it can be
    /// logged / surfaced in telemetry without an extra read.
    pub sidecar: VadSidecar,
}

/// Apply VAD to `input_wav` and write `<stem>.speech.wav` +
/// `<stem>.vad.json` next to it. Idempotent: if the source contains
/// no detectable speech the speech wav is written as a 0-sample file
/// and the sidecar records zero ranges — callers can treat that the
/// same way they treat "no audio".
pub fn apply_vad_to_wav(input_wav: &Path) -> Result<VadFilterOutcome> {
    apply_vad_to_wav_with(input_wav, VadEngine::default())
}

/// Same as [`apply_vad_to_wav`] but lets the caller pin the detector
/// engine. Used by the `run_vad` Tauri command when the user has
/// chosen the RMS fallback in Settings, and by unit tests that need
/// to exercise both paths without flipping global state.
pub fn apply_vad_to_wav_with(input_wav: &Path, engine: VadEngine) -> Result<VadFilterOutcome> {
    let stem = input_wav
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            AttuneError::Transcription(format!(
                "vad: input path {} has no usable stem",
                input_wav.display()
            ))
        })?
        .to_string();
    apply_vad_to_wav_with_stem(input_wav, engine, &stem)
}

/// Same as [`apply_vad_to_wav_with`] but writes the output artefacts
/// under an explicit `out_stem` instead of deriving them from the input
/// filename.
///
/// This exists so a pre-processing pass (e.g. the system-audio speech
/// enhancement in [`crate::audio::enhancement`], which writes
/// `system.enhanced.wav`) can feed its product through VAD while the
/// downstream `<out_stem>.speech.wav` + `<out_stem>.vad.json` keep their
/// canonical channel names (`system.speech.wav`), so the transcription
/// step finds them unchanged.
pub fn apply_vad_to_wav_with_stem(
    input_wav: &Path,
    engine: VadEngine,
    out_stem: &str,
) -> Result<VadFilterOutcome> {
    let reader = WavReader::open(input_wav).map_err(|e| {
        AttuneError::Transcription(format!("vad: could not open {}: {e}", input_wav.display()))
    })?;
    let spec = reader.spec();
    let source_samples = read_interleaved_f32(reader)?;
    let original_frame_count = (source_samples.len() / spec.channels.max(1) as usize) as u64;

    // VAD operates on 16 kHz mono. Resample a copy if the source isn't
    // already in that shape; the source itself is untouched.
    let mono16k = to_mono_16k(&source_samples, spec.channels, spec.sample_rate)?;

    // Detector dispatch. Silero is the default since 2026-05-27. If
    // Silero fails to initialise for some reason (sandbox, missing
    // ONNX runtime symbols), we fall back to the RMS gate at runtime
    // and log — the user gets a coarser cut but never a hard failure.
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

    // Translate the 16 kHz mono ranges back into source-sample-rate
    // ranges so we cut the source WAV (not the resampled copy).
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

    // Write the compacted WAV preserving the source format.
    let speech_silence_frames =
        ((spec.sample_rate as u64 * SILENCE_PAD_MS) as f64 / 1000.0).round() as u64;
    let mut writer = WavWriter::create(&speech_path, spec).map_err(|e| {
        AttuneError::Transcription(format!(
            "vad: could not create {}: {e}",
            speech_path.display()
        ))
    })?;
    let channels = spec.channels as usize;
    let mut cut_cursor: u64 = 0;
    let mut mappings: Vec<VadRangeMapping> = Vec::with_capacity(source_ranges.len());
    let pad_sample = make_pad_sample(spec.sample_format);

    for (i, range) in source_ranges.iter().enumerate() {
        // Inter-range silence so whisper sees natural utterance
        // boundaries between the joined active regions.
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
        // Slice the source frames + write to the compacted file.
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
        AttuneError::Transcription(format!(
            "vad: finalising {} failed: {e}",
            speech_path.display()
        ))
    })?;

    let original_seconds = original_frame_count as f64 / spec.sample_rate as f64;
    let kept_seconds = cut_cursor as f64 / spec.sample_rate as f64;
    // The padding silence inserted between ranges is part of `kept_samples`
    // on purpose — it's what the ASR will see. But for "silence stripped"
    // telemetry we want to report the real saving versus the original.
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
        .map_err(|e| AttuneError::Transcription(format!("vad: serialising sidecar failed: {e}")))?;
    std::fs::write(&sidecar_path, sidecar_json).map_err(|e| {
        AttuneError::Transcription(format!(
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

/// Convert a timestamp in the compacted-WAV timeline back to the
/// original-WAV timeline using the sidecar's range mapping. Used by
/// the transcription pipeline so segment timestamps the ASR emits
/// (which it timed against the cut audio) still line up with the
/// original recording's playhead.
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
    // Sample falls inside a padding-silence gap between two ranges —
    // remap to the start of the next range so the timestamp lands on
    // real audio rather than the synthetic silence.
    if let Some(next) = sidecar
        .ranges
        .iter()
        .find(|r| r.cut_start_sample > cut_sample)
    {
        return next.original_start_sample as f64 / sidecar.sample_rate as f64;
    }
    // Beyond the last range — clamp to the end of the original audio.
    sidecar.original_samples as f64 / sidecar.sample_rate as f64
}

/// Widen each active range by `pad_ms` on each side, clamp to [0,
/// total_samples], and merge any newly-overlapping neighbours.
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
                    AttuneError::Transcription(format!("vad: wav read failed: {e}"))
                })?);
            }
        }
        SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            if !(8..=32).contains(&bits) {
                return Err(AttuneError::Transcription(format!(
                    "vad: unsupported PCM bit depth: {bits}"
                )));
            }
            let max = (1i64 << (bits - 1)) as f32;
            for s in reader.into_samples::<i32>() {
                let raw = s.map_err(|e| {
                    AttuneError::Transcription(format!("vad: wav read failed: {e}"))
                })?;
                out.push(raw as f32 / max);
            }
        }
    }
    Ok(out)
}

fn make_pad_sample(_format: SampleFormat) -> f32 {
    // Pad with true digital silence; format conversion happens in
    // `write_sample_f32`.
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
            .map_err(|e| AttuneError::Transcription(format!("vad: wav write failed: {e}"))),
        SampleFormat::Int => {
            let spec = writer.spec();
            let bits = spec.bits_per_sample.max(1);
            let max = (1i64 << (bits - 1)) as f32;
            let clamped = value.clamp(-1.0, 1.0);
            // Clamp the integer to the signed range: +1.0 * 2^(bits-1) is
            // one past i16::MAX and hound rejects it as `TooWide`, which
            // would fail the whole VAD write.
            let lo = -(1i64 << (bits - 1)) as i32;
            let hi = ((1i64 << (bits - 1)) - 1) as i32;
            let int_sample = ((clamped * max).round() as i32).clamp(lo, hi);
            writer
                .write_sample(int_sample)
                .map_err(|e| AttuneError::Transcription(format!("vad: wav write failed: {e}")))
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
        // Both engines must agree that pure silence yields nothing.
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

    // The next two tests exercise the *algorithmic* behaviour around
    // pad-and-merge ranges using sine-wave fixtures, which the RMS
    // gate accepts as "loud signal". Silero correctly rejects sine
    // as non-speech (that's the whole reason we migrated to it), so
    // they are pinned to the RMS engine. Silero's own speech-vs-non-
    // speech behaviour is covered by the unit tests in
    // `audio::vad::silero::tests`.

    #[test]
    fn rms_keeps_a_pure_loud_signal_in_full() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mic.wav");
        write_test_wav(&path, 16_000, 1, &loud_sine(16_000 * 30, 440, 16_000));

        let outcome = apply_vad_to_wav_with(&path, VadEngine::Rms).unwrap();
        assert_eq!(outcome.sidecar.ranges.len(), 1);
        // Padding can widen but never beyond the source.
        assert!(outcome.sidecar.kept_samples >= 16_000 * 30);
        assert!(outcome.sidecar.active_ratio > 0.99);
    }

    #[test]
    fn rms_collapses_silence_between_loud_islands() {
        // 30 s loud · 60 s silent · 30 s loud → expect two ranges,
        // ~60 s saved minus a 300 ms inter-range pad.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mic.wav");
        let mut buf = loud_sine(16_000 * 30, 440, 16_000);
        buf.extend(std::iter::repeat_n(0.0_f32, 16_000 * 60));
        buf.extend(loud_sine(16_000 * 30, 440, 16_000));
        write_test_wav(&path, 16_000, 1, &buf);

        let outcome = apply_vad_to_wav_with(&path, VadEngine::Rms).unwrap();
        assert_eq!(outcome.sidecar.ranges.len(), 2);
        // ~60 s of silence dropped. The exact saving depends on how
        // the RMS gate aligns to the 30 s windows and how aggressive
        // the padding is; assert a generous lower bound.
        assert!(outcome.sidecar.silence_stripped_seconds > 30.0);
    }

    /// Side-by-side comparison of Silero vs RMS on a real on-disk
    /// recording. Skipped in CI; run with
    ///
    ///   cargo test --release -p attune-core --lib -- \
    ///       --ignored audio::vad_filter::tests::compare_engines_on_recording
    ///
    /// after pointing `ATTUNE_VAD_FIXTURE` at a session directory
    /// containing `mic.wav` and `system.wav`. Prints engine /
    /// ranges / kept / stripped / inference-time for both channels.
    /// Used as the migration acceptance harness — the Silero numbers
    /// should match the user's intuition about how much actual
    /// speech the recording contains.
    #[test]
    #[ignore = "requires ATTUNE_VAD_FIXTURE=<session_dir>"]
    fn compare_engines_on_recording() {
        let fixture = match std::env::var("ATTUNE_VAD_FIXTURE") {
            Ok(s) => std::path::PathBuf::from(s),
            Err(_) => panic!(
                "set ATTUNE_VAD_FIXTURE to a session directory containing mic.wav / system.wav"
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
        // The migration's whole point: Silero must say "not speech"
        // for a loud non-speech signal. RMS would have falsely kept
        // it; Silero produces 0 ranges, ~100% of the audio stripped.
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
        // 15s into the cut audio = 15s into the original (first range
        // is the head of both timelines).
        assert!((remap_cut_seconds_to_original(&sidecar, 15.0) - 15.0).abs() < 0.01);
        // 45s into the cut audio = 60 + (45-30) = 75s into the
        // original (we're 15s into the second range).
        assert!((remap_cut_seconds_to_original(&sidecar, 45.0) - 75.0).abs() < 0.01);
    }
}
