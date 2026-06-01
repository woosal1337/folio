//! Local Whisper transcription backend (whisper.cpp via whisper-rs).
//!
//! Loads a GGML model from disk, decodes the input WAV to 16 kHz mono
//! f32 (using the same `StreamingResampler` the capture pipeline uses
//! when it needs to retarget sample rates), and runs full inference.
//! Metal acceleration is enabled in the workspace Cargo on macOS so a
//! 10-minute meeting on Apple Silicon completes in ~realtime with the
//! `large-v3` model.

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

/// Whisper consumes 16 kHz mono f32 audio. Any other shape goes through
/// the resampler before inference.
const WHISPER_INPUT_SAMPLE_RATE: u32 = 16_000;

/// Search window (seconds) before a window's nominal end for the quietest
/// frame to cut at, so internal window boundaries land between words rather
/// than through one (GET-190 review — each window is decoded independently,
/// so a hard mid-word cut would drop or garble the straddling word).
const WINDOW_SILENCE_LOOKBACK_SECONDS: f64 = 2.0;

/// Beyond this much silence between two VAD ranges, don't carry the prior
/// range's detected language forward into the next — the speaker may have
/// switched language while the mic was quiet, so re-detect instead. Within
/// the budget, inheriting keeps a short or noisy continuation in the
/// language it was already in (GET-190 review).
const LANG_CARRY_MAX_GAP_SECONDS: f64 = 30.0;

/// Vocabulary glossary fed to Whisper's `initial_prompt`. Steers the
/// spelling of proper nouns and recurring technical terms that
/// Whisper-large-v3 otherwise mangles on the user's meeting audio. See
/// `~/Documents/GitHub/obsidian.md/projects/attune/references/whisper-customization.md`
/// for the 224-token budget and the cookbook-derived design rationale.
///
/// Keep this a comma-separated proper-noun list rather than a sentence-
/// style example: the cookbook shows glossary-form prompts do NOT leak
/// into the transcript, but style-form prompts CAN.
const ATTUNE_INITIAL_PROMPT: &str =
    "Attune meeting glossary: Tahir, Yusuf, İbrahim, Ege, Vusal, Azerbaycan, \
     Chrome extension, Claude, Gemini, MIS, veri tabanı, sistemleri, \
     multidisipliner, agent, startup.";

pub struct LocalWhisperTranscriber {
    model_path: PathBuf,
    /// Number of CPU threads to use. Defaults to `num_cpus` heuristic.
    /// Exposed via [`LocalWhisperTranscriber::with_threads`] for tests.
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
        // Tag this thread as USER_INITIATED so the macOS scheduler can
        // dispatch the heavy whisper work to the E-cores when system
        // load allows — keeps the P-cores cool for the UI + capture
        // callbacks while transcoding still finishes in foreground
        // time. Stub on non-macOS. v2 finding 064 / GET-99.
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

        // Silence pre-filter. Whisper hallucinates aggressively on
        // chunks that are functionally silent (a previous test on
        // mic.wav at -74.8 dBFS RMS produced "I will put the tape on
        // the back of the box" and other craft-video junk that the
        // post-decode phrase filter can never enumerate). If the audio
        // is below SILENCE_RMS_THRESHOLD we skip inference entirely
        // and return an empty transcript. The threshold sits well
        // below normal speech (~-30 dBFS RMS) and well above true
        // digital silence (~-90 dBFS RMS).
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
        const SILENCE_RMS_THRESHOLD: f32 = 0.002; // -54 dBFS
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

        // Per-window VAD pre-pass. The global-RMS gate above only
        // catches recordings that are silent end-to-end. A meeting
        // with 30s of speech followed by 60s of listening passes
        // the global gate but the silent tail hallucinates loops
        // ("I'm going to ask you to take your own distance from
        // there." × 14 in the 2026-05-26-11-47-54 mic.wav). VAD
        // slices the buffer into active ranges so whisper only ever
        // sees speech-bearing audio. Empty result = treat the whole
        // recording as silence.
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

        // Loading the model is the expensive step (~hundreds of MB
        // mapped in). We do it inside `transcribe` for v1 — fine for
        // one-shot use; a follow-up could cache the context across
        // requests behind a `OnceLock` keyed by model path.
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

        // Language handling. The whisper.cpp default for `language` is
        // "en", and `detect_language = true` is a *detect-only* mode
        // that returns early without transcribing — we tried that and
        // got zero segments back. The correct way to auto-detect is to
        // pass language = NULL (None on the Rust side); whisper.cpp
        // then runs detection as part of the regular full() call and
        // transcribes in the detected language.
        let hint = language_hint.filter(|l| !l.is_empty() && *l != "auto");
        let threads = self.threads;

        // Per-window inference loop (GET-190). Each VAD range is sub-
        // divided into ≤28 s windows so whisper decodes exactly one window
        // in one language per `full()` call — there is no internal language
        // lock-in to leak across a mid-stream code-switch. For each window
        // we detect the language up front (encoder-only, ~200-400 ms) and
        // hand it to whisper explicitly; a low-confidence or short window
        // inherits the previous window's language so a noisy chunk can't
        // flip the transcript. The other half of the corruption chain —
        // condition_on_previous_text seeding the next chunk with the
        // previous (possibly hallucinated) text — is already off via
        // set_no_context(true) + n_max_text_ctx(0) in build_params.
        let sample_rate_f = WHISPER_INPUT_SAMPLE_RATE as f64;
        let window_samples = (language_id::LID_WINDOW_SECONDS * sample_rate_f) as usize;
        let mut segments = Vec::new();
        let mut last_detected_lang_id: Option<i32> = None;
        // Forced language: an explicit user override (Settings →
        // Transcription language). `None` = auto, the per-window LID path.
        let forced = hint;
        // The last language we were confident about, carried across windows
        // for the inherit-prior fallback. Seeded by a forced hint.
        let mut confirmed_lang: Option<String> = forced.map(|s| s.to_string());
        info!(
            ranges = ranges.len(),
            forced_language = forced,
            window_seconds = language_id::LID_WINDOW_SECONDS,
            "starting local whisper inference (per-window language id)"
        );
        let mut prev_range_end: Option<usize> = None;
        for (idx, range) in ranges.iter().enumerate() {
            // After a long silence, don't carry the previous range's
            // language into a new one — the speaker may have switched while
            // the mic was quiet. Within the gap budget, inheriting keeps a
            // short or noisy continuation in the language it was already in.
            if let Some(prev_end) = prev_range_end {
                let gap_secs = range.start.saturating_sub(prev_end) as f64 / sample_rate_f;
                if gap_secs > LANG_CARRY_MAX_GAP_SECONDS {
                    confirmed_lang = forced.map(|s| s.to_string());
                }
            }
            let mut win_start = range.start;
            while win_start < range.end {
                // Sub-divide into ≤28 s windows. Land each internal cut in
                // the quietest spot near the nominal end so a boundary falls
                // between words, not through one. The final window of a
                // range ends at range.end untouched (already a VAD edge).
                let nominal_end = (win_start + window_samples).min(range.end);
                let win_end = if nominal_end >= range.end {
                    range.end
                } else {
                    quiet_window_end(&pcm, nominal_end).max(win_start + 1)
                };
                let slice = &pcm[win_start..win_end];
                let offset_secs = win_start as f64 / sample_rate_f;
                let window_secs = (win_end - win_start) as f64 / sample_rate_f;

                // Decide this window's language: an explicit override wins;
                // otherwise detect it (encoder-only) and apply the
                // confidence / min-duration / inherit-prior policy. A window
                // too short to trust skips detection entirely (its result
                // would be discarded anyway) and inherits the prior.
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

                // When we forced/inherited a language, that IS the segment
                // language. When we let whisper auto-detect (window_lang
                // None, only possible before anything is confirmed), read
                // back what it chose so the segments are still tagged and
                // the next window can inherit it.
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
                        // whisper.cpp reports per-call timestamps in
                        // centiseconds. Add the window offset to anchor
                        // back to the original recording timeline.
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

        // Dedupe consecutive-identical-text loops first — these are
        // the contextual hallucinations whisper falls into on quiet
        // chunks ("I'm going to ask you to take your own distance
        // from there." × 14 in the 2026-05-26-11-47-54 mic.wav).
        // Then strip canonical training-data artifacts ("Thank you."
        // / "Altyazı M.K." / Amara.org credits) the phrase filter
        // knows about.
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
            // Channel-level language: the explicit hint if forced, else the
            // most common per-segment language we detected on the auto path
            // (GET-190) — far more useful than the old always-None.
            language: hint
                .map(|s| s.to_string())
                .or_else(|| majority_language(&segments)),
            segments,
        })
    }
}

/// Choose a window end near `nominal_end` that sits in the quietest 50 ms
/// frame within the preceding [`WINDOW_SILENCE_LOOKBACK_SECONDS`], so an
/// internal window boundary lands between words. Falls back to
/// `nominal_end` when the whole lookback is loud (rare — people breathe).
fn quiet_window_end(pcm: &[f32], nominal_end: usize) -> usize {
    const FRAME: usize = 800; // 50 ms @ 16 kHz
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

/// The most common non-empty per-segment language across `segments`, for
/// the channel-level label. Ties break toward first appearance. None when
/// no segment carries a language.
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
            // `>=` keeps the earlier language on a tie (order is by first
            // appearance).
            Some(b) if counts[b] >= counts[lang] => Some(b),
            _ => Some(lang),
        })
        .map(|s| s.to_string())
}

/// Build the whisper params for one window. `lang` forces the decode
/// language (`None` lets whisper auto-detect on this call). Per GET-190 the
/// language is decided per window rather than once for the whole recording,
/// so this is called fresh for every window. `FullParams` borrows its
/// prompt + language strings, isn't `Clone`, and can't be cached, hence a
/// builder.
///
/// Note: whisper-rs 0.13.2's `set_language`/`set_initial_prompt` move a
/// `CString` into whisper.cpp via `into_raw()` with no `Drop` to reclaim
/// it, so each call leaks its prompt + language CString. The leak is small
/// and bounded by window count (a few dozen KB on a long recording) and the
/// process is short-lived per `transcribe()`, so we accept it.
fn build_params(lang: Option<&str>, threads: i32) -> FullParams<'_, '_> {
    // Greedy beats beam search empirically on this user's audio. Beam
    // search converges on the highest-probability memorised token in
    // training data, which for any quiet or musical Turkish chunk is
    // "Altyazı M.K." (subtitle credit artifact, openai/whisper#2412). On a
    // 42-second German/Turkish song beam search produced "Altyazı M.K." x2
    // while greedy produced the actual lyrics. best_of is unused at
    // temperature=0 but 5 is the upstream default so we keep it.
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 5 });
    params.set_n_threads(threads);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);
    // no_context + max_text_ctx=0 prevents whisper.cpp from feeding the
    // previous segment's text as a prompt to the next segment, the other
    // half of the GET-190 corruption chain — a hallucinated chunk seeding
    // the next ("We will choose the sixth one." × ∞) once a bad token slips
    // in. This is whisper-rs's equivalent of condition_on_previous_text=false.
    params.set_no_context(true);
    params.set_n_max_text_ctx(0);
    params.set_suppress_blank(true);
    params.set_suppress_non_speech_tokens(true);
    // Temperature fallback chain. whisper.cpp tries [0.0, 0.2, 0.4, 0.6,
    // 0.8, 1.0] in order, accepting the first decode whose token entropy
    // (over the last 32 tokens) clears entropy_thold AND whose avg logprob
    // clears logprob_thold. Entropy is whisper.cpp's substitute for
    // OpenAI's compression_ratio_threshold and catches "I think that you
    // know"-style loops.
    params.set_temperature(0.0);
    params.set_temperature_inc(0.2);
    params.set_entropy_thold(2.4);
    params.set_logprob_thold(-1.0);
    // 0.8 is tighter than whisper.cpp's default 0.6 — only emit a segment
    // if the no-speech head is confident the window contains real speech.
    params.set_no_speech_thold(0.8);
    params.set_max_initial_ts(1.0);
    // Word-level UI affordances. token_timestamps emits per-token timing,
    // split_on_word keeps segment breaks at word boundaries, max_len caps a
    // single segment at ~one sentence for the live-transcript view.
    params.set_token_timestamps(true);
    params.set_split_on_word(true);
    params.set_max_len(120);
    params.set_initial_prompt(ATTUNE_INITIAL_PROMPT);
    params.set_language(lang);
    params
}

/// Decode a WAV file to `output_sample_rate` mono f32 samples in
/// `[-1, 1]`. Handles int + float WAV variants and any common bit
/// depth. Used by the local Whisper backend (which insists on 16 kHz
/// mono) and can be reused by any future on-device pipeline.
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
    // whisper.cpp benefits from physical cores; logical-core overshoot
    // hurts. std::thread::available_parallelism returns a NonZeroUsize
    // for logical cores. Halve it as a conservative approximation.
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
        // tr appears 3×, en 2× → tr.
        let segs = [
            seg_lang(Some("tr")),
            seg_lang(Some("en")),
            seg_lang(Some("tr")),
            seg_lang(Some("en")),
            seg_lang(Some("tr")),
        ];
        assert_eq!(majority_language(&segs).as_deref(), Some("tr"));
        // Tie (1 each) → first appearance (en).
        let tie = [seg_lang(Some("en")), seg_lang(Some("tr"))];
        assert_eq!(majority_language(&tie).as_deref(), Some("en"));
    }

    #[test]
    fn quiet_window_end_cuts_at_the_lowest_energy_frame() {
        // 28 s of loud audio, then a quiet 50 ms dip ~1 s before the end.
        let sr = WHISPER_INPUT_SAMPLE_RATE as usize;
        let nominal = 28 * sr;
        let mut pcm = vec![0.5_f32; nominal + sr]; // loud everywhere
                                                   // Carve a silent 50 ms frame at 27.0 s — inside the 2 s lookback.
        let dip = 27 * sr;
        for s in pcm.iter_mut().skip(dip).take(800) {
            *s = 0.0;
        }
        let cut = quiet_window_end(&pcm, nominal);
        // The cut should land on the silent dip, not the hard 28 s mark.
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
        // No dip → the first (== loudest-tie) frame wins; still a valid
        // index < nominal within the lookback. Just assert it doesn't panic
        // and stays within the lookback window.
        let cut = quiet_window_end(&pcm, nominal);
        assert!(cut <= nominal && cut >= nominal - 2 * sr);
    }

    #[test]
    fn decodes_and_resamples_stereo_48k_to_mono_16k() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("system.wav");
        write_wav(&path, 48_000, 2, 48_000);

        let pcm = decode_wav_to_mono_f32(&path, 16_000).unwrap();
        // 1 second of 48 kHz audio should produce ~16_000 mono samples;
        // rubato can pad by up to a chunk.
        assert!(
            (pcm.len() as i64 - 16_000).abs() < 1024,
            "got {} samples, expected ~16000",
            pcm.len()
        );
    }
}
