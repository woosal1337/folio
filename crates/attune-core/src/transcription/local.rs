use std::path::{Path, PathBuf};

use hound::WavReader;
use tracing::{debug, info};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::audio::resampler::StreamingResampler;
use crate::error::{AttuneError, Result};
use crate::qos::{set_thread_qos, QosClass};
use crate::transcription::hallucination_filter::{dedupe_repetitions, filter_segments};
use crate::transcription::language_id;
use crate::transcription::vad::active_ranges;
use crate::transcription::{Transcriber, Transcript, TranscriptSegment};

const WHISPER_INPUT_SAMPLE_RATE: u32 = 16_000;

const WINDOW_SILENCE_LOOKBACK_SECONDS: f64 = 2.0;

const LANG_CARRY_MAX_GAP_SECONDS: f64 = 30.0;

const ATTUNE_INITIAL_PROMPT: &str =
    "Attune meeting glossary: Tahir, Yusuf, İbrahim, Ege, Vusal, Azerbaycan, \
     Chrome extension, Claude, Gemini, MIS, veri tabanı, sistemleri, \
     multidisipliner, agent, startup.";

pub struct LocalWhisperTranscriber {
    model_path: PathBuf,

    threads: i32,
}

impl LocalWhisperTranscriber {
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self {
            model_path: model_path.into(),
            threads: default_threads(),
        }
    }

    #[must_use]
    pub fn with_threads(mut self, threads: i32) -> Self {
        self.threads = threads.max(1);
        self
    }
}

impl Transcriber for LocalWhisperTranscriber {
    fn transcribe(&self, audio_path: &Path, language_hint: Option<&str>) -> Result<Transcript> {
        set_thread_qos(QosClass::UserInitiated);

        if !self.model_path.is_file() {
            return Err(AttuneError::Transcription(format!(
                "whisper model not found at {} — download it from Settings → Transcription",
                self.model_path.display()
            )));
        }

        debug!(
            model = %self.model_path.display(),
            audio = %audio_path.display(),
            threads = self.threads,
            "loading whisper model",
        );

        let pcm = decode_wav_to_mono_f32(audio_path, WHISPER_INPUT_SAMPLE_RATE)?;

        let rms = if pcm.is_empty() {
            0.0
        } else {
            (pcm.iter().map(|s| s * s).sum::<f32>() / pcm.len() as f32).sqrt()
        };
        let peak = pcm.iter().fold(0.0f32, |a, b| a.max(b.abs()));
        info!(
            samples = pcm.len(),
            rms, peak, "WAV decoded for whisper inference"
        );
        const SILENCE_RMS_THRESHOLD: f32 = 0.002;
        if rms < SILENCE_RMS_THRESHOLD {
            info!(
                rms,
                threshold = SILENCE_RMS_THRESHOLD,
                "audio below silence threshold, skipping whisper inference"
            );
            return Ok(Transcript {
                language: language_hint.map(|s| s.to_string()),
                segments: Vec::new(),
            });
        }

        let ranges = active_ranges(&pcm, WHISPER_INPUT_SAMPLE_RATE);
        if ranges.is_empty() {
            info!("vad: no active ranges, skipping whisper inference");
            return Ok(Transcript {
                language: language_hint.map(|s| s.to_string()),
                segments: Vec::new(),
            });
        }
        let active_samples: usize = ranges.iter().map(|r| r.end - r.start).sum();
        info!(
            ranges = ranges.len(),
            active_samples,
            total_samples = pcm.len(),
            active_ratio = active_samples as f32 / pcm.len().max(1) as f32,
            "vad: active ranges identified"
        );

        let whisper_context = WhisperContext::new_with_params(
            self.model_path
                .to_str()
                .ok_or_else(|| AttuneError::Transcription("non-UTF8 model path".into()))?,
            WhisperContextParameters::default(),
        )
        .map_err(|e| AttuneError::Transcription(format!("could not load whisper model: {e}")))?;

        let mut whisper_state = whisper_context
            .create_state()
            .map_err(|e| AttuneError::Transcription(format!("whisper state init: {e}")))?;

        let hint = language_hint.filter(|l| !l.is_empty() && *l != "auto");
        let threads = self.threads;

        let sample_rate_f = WHISPER_INPUT_SAMPLE_RATE as f64;
        let window_samples = (language_id::LID_WINDOW_SECONDS * sample_rate_f) as usize;
        let mut segments = Vec::new();
        let mut last_detected_lang_id: Option<i32> = None;

        let forced = hint;

        let mut confirmed_lang: Option<String> = forced.map(|s| s.to_string());
        info!(
            ranges = ranges.len(),
            forced_language = forced,
            window_seconds = language_id::LID_WINDOW_SECONDS,
            "starting local whisper inference (per-window language id)"
        );
        let mut prev_range_end: Option<usize> = None;
        for (idx, range) in ranges.iter().enumerate() {
            if let Some(prev_end) = prev_range_end {
                let gap_secs = range.start.saturating_sub(prev_end) as f64 / sample_rate_f;
                if gap_secs > LANG_CARRY_MAX_GAP_SECONDS {
                    confirmed_lang = forced.map(|s| s.to_string());
                }
            }
            let mut win_start = range.start;
            while win_start < range.end {
                let nominal_end = (win_start + window_samples).min(range.end);
                let win_end = if nominal_end >= range.end {
                    range.end
                } else {
                    quiet_window_end(&pcm, nominal_end).max(win_start + 1)
                };
                let slice = &pcm[win_start..win_end];
                let offset_secs = win_start as f64 / sample_rate_f;
                let window_secs = (win_end - win_start) as f64 / sample_rate_f;

                let window_lang: Option<String> = match forced {
                    Some(f) => Some(f.to_string()),
                    None => {
                        let det = if window_secs >= language_id::MIN_LID_SECONDS {
                            language_id::detect_language(
                                &mut whisper_state,
                                slice,
                                threads as usize,
                            )
                        } else {
                            None
                        };
                        let (lang, confirmed) = language_id::resolve_window_language(
                            det.as_ref(),
                            window_secs,
                            confirmed_lang.as_deref(),
                        );
                        if confirmed.is_some() {
                            confirmed_lang = confirmed;
                        }
                        debug!(
                            range_idx = idx,
                            offset_secs,
                            window_secs,
                            detected = det.as_ref().and_then(|d| d.code.clone()),
                            confidence = det.as_ref().map(|d| d.confidence),
                            chosen = lang.as_deref(),
                            "lid: window language"
                        );
                        lang
                    }
                };

                whisper_state
                    .full(build_params(window_lang.as_deref(), threads), slice)
                    .map_err(|e| AttuneError::Transcription(format!("whisper full(): {e}")))?;

                let resolved_lang: Option<String> = window_lang.clone().or_else(|| {
                    whisper_state
                        .full_lang_id_from_state()
                        .ok()
                        .and_then(|id| whisper_rs::get_lang_str(id).map(|s| s.to_string()))
                });
                if confirmed_lang.is_none() {
                    confirmed_lang = resolved_lang.clone();
                }
                if let Ok(lang_id) = whisper_state.full_lang_id_from_state() {
                    last_detected_lang_id = Some(lang_id);
                }

                let n = whisper_state
                    .full_n_segments()
                    .map_err(|e| AttuneError::Transcription(format!("whisper segments: {e}")))?;
                for i in 0..n {
                    let text = whisper_state
                        .full_get_segment_text(i)
                        .map_err(|e| AttuneError::Transcription(format!("segment text: {e}")))?;
                    let t0 = whisper_state
                        .full_get_segment_t0(i)
                        .map_err(|e| AttuneError::Transcription(format!("segment t0: {e}")))?;
                    let t1 = whisper_state
                        .full_get_segment_t1(i)
                        .map_err(|e| AttuneError::Transcription(format!("segment t1: {e}")))?;
                    segments.push(TranscriptSegment {
                        start_seconds: offset_secs + t0 as f64 / 100.0,
                        end_seconds: offset_secs + t1 as f64 / 100.0,
                        text: text.trim().to_string(),
                        speaker: None,
                        language: resolved_lang.clone(),
                    });
                }

                win_start = win_end;
            }
            prev_range_end = Some(range.end);
        }

        let (segments, looped) = dedupe_repetitions(segments);
        if !looped.is_empty() {
            info!(
                count = looped.len(),
                sample = ?looped.iter().take(3).collect::<Vec<_>>(),
                "dropped whisper repetition loops",
            );
        }
        let (segments, dropped_hallucinations) = filter_segments(segments);
        if !dropped_hallucinations.is_empty() {
            info!(
                count = dropped_hallucinations.len(),
                dropped = ?dropped_hallucinations,
                "filtered whisper hallucinations",
            );
        }

        info!(
            segments = segments.len(),
            dropped_hallucinations = dropped_hallucinations.len(),
            dropped_repetitions = looped.len(),
            detected_lang_id = last_detected_lang_id,
            "local whisper inference complete"
        );

        Ok(Transcript {
            language: hint
                .map(|s| s.to_string())
                .or_else(|| majority_language(&segments)),
            segments,
        })
    }
}

fn quiet_window_end(pcm: &[f32], nominal_end: usize) -> usize {
    const FRAME: usize = 800;
    let lookback = (WINDOW_SILENCE_LOOKBACK_SECONDS * WHISPER_INPUT_SAMPLE_RATE as f64) as usize;
    let lo = nominal_end.saturating_sub(lookback);
    let mut best = nominal_end;
    let mut best_rms = f32::MAX;
    let mut f = lo;
    while f + FRAME <= nominal_end {
        let frame = &pcm[f..f + FRAME];
        let rms = (frame.iter().map(|s| s * s).sum::<f32>() / FRAME as f32).sqrt();
        if rms < best_rms {
            best_rms = rms;
            best = f;
        }
        f += FRAME;
    }
    best
}

fn majority_language(segments: &[TranscriptSegment]) -> Option<String> {
    use std::collections::HashMap;
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut order: Vec<&str> = Vec::new();
    for s in segments {
        if let Some(lang) = s.language.as_deref() {
            if !counts.contains_key(lang) {
                order.push(lang);
            }
            *counts.entry(lang).or_insert(0) += 1;
        }
    }
    order
        .into_iter()
        .fold(None, |best: Option<&str>, lang| match best {
            Some(b) if counts[b] >= counts[lang] => Some(b),
            _ => Some(lang),
        })
        .map(|s| s.to_string())
}

fn build_params(lang: Option<&str>, threads: i32) -> FullParams<'_, '_> {
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 5 });
    params.set_n_threads(threads);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);

    params.set_no_context(true);
    params.set_n_max_text_ctx(0);
    params.set_suppress_blank(true);
    params.set_suppress_non_speech_tokens(true);

    params.set_temperature(0.0);
    params.set_temperature_inc(0.2);
    params.set_entropy_thold(2.4);
    params.set_logprob_thold(-1.0);

    params.set_no_speech_thold(0.8);
    params.set_max_initial_ts(1.0);

    params.set_token_timestamps(true);
    params.set_split_on_word(true);
    params.set_max_len(120);
    params.set_initial_prompt(ATTUNE_INITIAL_PROMPT);
    params.set_language(lang);
    params
}

pub(crate) fn decode_wav_to_mono_f32(
    audio_path: &Path,
    output_sample_rate: u32,
) -> Result<Vec<f32>> {
    let reader = WavReader::open(audio_path).map_err(|e| {
        AttuneError::Transcription(format!(
            "could not open audio file {}: {e}",
            audio_path.display()
        ))
    })?;
    let spec = reader.spec();
    let samples = read_samples_as_f32(reader)?;

    let needs_resample = spec.sample_rate != output_sample_rate || spec.channels != 1;
    if !needs_resample {
        return Ok(samples);
    }

    let mut resampler =
        StreamingResampler::new(spec.sample_rate, spec.channels, output_sample_rate)?;
    let mut out = resampler.process(&samples)?;
    out.extend(resampler.flush()?);
    Ok(out)
}

fn read_samples_as_f32<R: std::io::Read>(reader: WavReader<R>) -> Result<Vec<f32>> {
    let spec = reader.spec();
    let mut out = Vec::with_capacity(reader.len() as usize);

    match spec.sample_format {
        hound::SampleFormat::Float => {
            for sample in reader.into_samples::<f32>() {
                let s = sample
                    .map_err(|e| AttuneError::Transcription(format!("wav sample decode: {e}")))?;
                out.push(s);
            }
        }
        hound::SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            if !(8..=32).contains(&bits) {
                return Err(AttuneError::Transcription(format!(
                    "unsupported PCM bit depth: {bits}"
                )));
            }
            let max = (1i64 << (bits - 1)) as f32;
            for sample in reader.into_samples::<i32>() {
                let s = sample
                    .map_err(|e| AttuneError::Transcription(format!("wav sample decode: {e}")))?;
                out.push(s as f32 / max);
            }
        }
    }
    Ok(out)
}

fn default_threads() -> i32 {
    std::thread::available_parallelism()
        .map(|p| (p.get() / 2).max(1) as i32)
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_wav(path: &Path, sample_rate: u32, channels: u16, samples: u32) {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..(samples * channels as u32) {
            let v = ((i as f32 * 0.01).sin() * 0.1 * i16::MAX as f32) as i16;
            writer.write_sample(v).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn errors_when_model_missing() {
        let dir = TempDir::new().unwrap();
        let model = dir.path().join("nope.bin");
        let audio = dir.path().join("mic.wav");
        write_wav(&audio, 16_000, 1, 16_000);

        let transcriber = LocalWhisperTranscriber::new(model);
        let err = transcriber.transcribe(&audio, None).unwrap_err();
        assert!(matches!(err, AttuneError::Transcription(_)));
    }

    #[test]
    fn decodes_passthrough_when_format_matches() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mic.wav");
        write_wav(&path, 16_000, 1, 8_000);

        let pcm = decode_wav_to_mono_f32(&path, 16_000).unwrap();
        assert_eq!(pcm.len(), 8_000);
    }

    fn seg_lang(lang: Option<&str>) -> TranscriptSegment {
        TranscriptSegment {
            start_seconds: 0.0,
            end_seconds: 1.0,
            text: "x".into(),
            speaker: None,
            language: lang.map(str::to_string),
        }
    }

    #[test]
    fn majority_language_picks_the_mode_then_first_on_ties() {
        assert_eq!(majority_language(&[]), None);
        assert_eq!(majority_language(&[seg_lang(None), seg_lang(None)]), None);

        let segs = [
            seg_lang(Some("tr")),
            seg_lang(Some("en")),
            seg_lang(Some("tr")),
            seg_lang(Some("en")),
            seg_lang(Some("tr")),
        ];
        assert_eq!(majority_language(&segs).as_deref(), Some("tr"));

        let tie = [seg_lang(Some("en")), seg_lang(Some("tr"))];
        assert_eq!(majority_language(&tie).as_deref(), Some("en"));
    }

    #[test]
    fn quiet_window_end_cuts_at_the_lowest_energy_frame() {
        let sr = WHISPER_INPUT_SAMPLE_RATE as usize;
        let nominal = 28 * sr;
        let mut pcm = vec![0.5_f32; nominal + sr];

        let dip = 27 * sr;
        for s in pcm.iter_mut().skip(dip).take(800) {
            *s = 0.0;
        }
        let cut = quiet_window_end(&pcm, nominal);

        assert!(
            (cut as i64 - dip as i64).abs() < 800,
            "expected cut near the silent dip {dip}, got {cut}"
        );
    }

    #[test]
    fn quiet_window_end_falls_back_to_nominal_when_uniformly_loud() {
        let sr = WHISPER_INPUT_SAMPLE_RATE as usize;
        let nominal = 28 * sr;
        let pcm = vec![0.5_f32; nominal + sr];

        let cut = quiet_window_end(&pcm, nominal);
        assert!(cut <= nominal && cut >= nominal - 2 * sr);
    }

    #[test]
    fn decodes_and_resamples_stereo_48k_to_mono_16k() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("system.wav");
        write_wav(&path, 48_000, 2, 48_000);

        let pcm = decode_wav_to_mono_f32(&path, 16_000).unwrap();

        assert!(
            (pcm.len() as i64 - 16_000).abs() < 1024,
            "got {} samples, expected ~16000",
            pcm.len()
        );
    }
}
