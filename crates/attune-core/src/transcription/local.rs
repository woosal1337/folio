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
use crate::transcription::hallucination_filter::filter_segments;
use crate::transcription::{Transcriber, Transcript, TranscriptSegment};

/// Whisper consumes 16 kHz mono f32 audio. Any other shape goes through
/// the resampler before inference.
const WHISPER_INPUT_SAMPLE_RATE: u32 = 16_000;

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

        // Loading the model is the expensive step (~hundreds of MB
        // mapped in). We do it inside `transcribe` for v1 — fine for
        // one-shot use; a follow-up could cache the context across
        // requests behind a `OnceLock` keyed by model path.
        let ctx = WhisperContext::new_with_params(
            self.model_path
                .to_str()
                .ok_or_else(|| AttuneError::Transcription("non-UTF8 model path".into()))?,
            WhisperContextParameters::default(),
        )
        .map_err(|e| AttuneError::Transcription(format!("could not load whisper model: {e}")))?;

        let mut state = ctx
            .create_state()
            .map_err(|e| AttuneError::Transcription(format!("whisper state init: {e}")))?;

        // Greedy beats beam search empirically on this user's audio. The
        // earlier "BeamSearch{5} is better in the literature" change was
        // a regression: beam search converges on the highest-probability
        // memorised token in the training data, which for any quiet or
        // musical Turkish chunk is "Altyazı M.K." (subtitle credit
        // artifact, see github.com/openai/whisper/discussions/2412). On
        // a 42-second German/Turkish song, beam search produced
        // "Altyazı M.K." x2 while greedy produced the actual lyrics.
        // On the user's meeting recording, greedy also matched the
        // previous shipping transcript word-for-word; beam search
        // mangled proper nouns. best_of is unused at temperature=0 but
        // 5 is the upstream default so we keep it.
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 5 });
        params.set_n_threads(self.threads);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);

        // Critical for transcript quality. By default whisper.cpp feeds
        // each segment the *previous* segment's text as a prompt. That
        // is great when the audio is clean and the previous text is
        // accurate; it is catastrophic when a hallucination slips in —
        // the bad text becomes the prompt for the next segment and the
        // model loops on its own output ("We will choose the sixth
        // one." × ∞). Turning context off makes each window decode
        // independently and prevents the cascade.
        //
        // Belt-and-suspenders: also clamp the max-text-context to 0 so
        // even if no_context were silently ignored on this build, no
        // previous tokens can sneak into the decoder prompt.
        params.set_no_context(true);
        params.set_n_max_text_ctx(0);

        // Strip whisper.cpp's annotation tokens like `[Music]` or
        // `[Inaudible]` from the output stream — they confuse the
        // downstream UI and rarely help.
        params.set_suppress_blank(true);
        params.set_suppress_non_speech_tokens(true);

        // Temperature fallback chain. whisper.cpp tries
        // [0.0, 0.2, 0.4, 0.6, 0.8, 1.0] in order, accepting the first
        // decode whose token entropy (over the last 32 tokens) clears
        // entropy_thold AND whose avg logprob clears logprob_thold.
        // Entropy is whisper.cpp's substitute for OpenAI's
        // compression_ratio_threshold and catches "I think that you
        // know"-style loops.
        params.set_temperature(0.0);
        params.set_temperature_inc(0.2);
        params.set_entropy_thold(2.4);
        params.set_logprob_thold(-1.0);

        // Tighten the silence guard a notch. The default 0.6 lets
        // mostly-silent windows through and they hallucinate
        // "training-data" English nonsense (the classic Roman-emperor
        // loop). 0.8 means whisper has to be more confident there is
        // actual speech before emitting a segment.
        params.set_no_speech_thold(0.8);

        // Prevent the decoder from jumping past the start of a chunk.
        // Without this, on chunks that start with quiet audio the model
        // sometimes emits its first segment with a timestamp several
        // seconds in, which corrupts the merged timeline.
        params.set_max_initial_ts(1.0);

        // Word-level UI affordances. token_timestamps emits per-token
        // timing metadata, split_on_word keeps segment breaks at word
        // boundaries, max_len caps a single segment at ~one sentence
        // for the live-transcript view. The text content of each
        // segment is unchanged — only how segment boundaries fall.
        params.set_token_timestamps(true);
        params.set_split_on_word(true);
        params.set_max_len(120);

        // Static glossary of proper nouns and Turkish technical
        // vocabulary the user's meetings repeat. Steers spelling on
        // names that Whisper-large-v3 otherwise approximates
        // ("multidisplinar" → "multidisipliner"). 224-token budget on
        // whisper.cpp's initial_prompt, so keep this terse.
        params.set_initial_prompt(ATTUNE_INITIAL_PROMPT);

        // Language handling. The whisper.cpp default for `language` is
        // "en", and `detect_language = true` is a *detect-only* mode
        // that returns early without transcribing — we tried that and
        // got zero segments back. The correct way to auto-detect is to
        // pass language = NULL (None on the Rust side); whisper.cpp
        // then runs detection as part of the regular full() call and
        // transcribes in the detected language.
        let hint = language_hint.filter(|l| !l.is_empty() && *l != "auto");
        params.set_language(hint);

        info!("starting local whisper inference");
        state
            .full(params, &pcm)
            .map_err(|e| AttuneError::Transcription(format!("whisper full(): {e}")))?;

        let n = state
            .full_n_segments()
            .map_err(|e| AttuneError::Transcription(format!("whisper segments: {e}")))?;

        let mut segments = Vec::with_capacity(n as usize);
        for i in 0..n {
            let text = state
                .full_get_segment_text(i)
                .map_err(|e| AttuneError::Transcription(format!("segment text: {e}")))?;
            let t0 = state
                .full_get_segment_t0(i)
                .map_err(|e| AttuneError::Transcription(format!("segment t0: {e}")))?;
            let t1 = state
                .full_get_segment_t1(i)
                .map_err(|e| AttuneError::Transcription(format!("segment t1: {e}")))?;
            segments.push(TranscriptSegment {
                // whisper.cpp reports timestamps in centiseconds (1/100 s).
                start_seconds: t0 as f64 / 100.0,
                end_seconds: t1 as f64 / 100.0,
                text: text.trim().to_string(),
            });
        }

        // Strip Whisper's "Thank you." / "you" / "Thanks for watching."
        // artifacts and the multilingual subtitle-credit hallucinations
        // ("Altyazı M.K.", "Amara.org community", "QTSS", …) that leak
        // through even after no_speech_thold=0.8. See
        // hallucination_filter for the curated list and the 2026-05
        // benchmark notes.
        //
        // Log every dropped text verbatim so that when a transcript
        // ends up unexpectedly short, we can tell whether the filter
        // caught real Whisper junk (good) or shadowed legitimate
        // speech (bug to fix).
        let (segments, dropped_hallucinations) = filter_segments(segments);
        if !dropped_hallucinations.is_empty() {
            info!(
                count = dropped_hallucinations.len(),
                dropped = ?dropped_hallucinations,
                "filtered whisper hallucinations",
            );
        }

        // Whisper exposes the detected language as an integer id into
        // its internal table. Log it so we can debug when transcripts
        // come back in the wrong language.
        let detected_lang_id = state.full_lang_id_from_state().ok();
        info!(
            segments = segments.len(),
            dropped_hallucinations = dropped_hallucinations.len(),
            detected_lang_id,
            "local whisper inference complete"
        );

        Ok(Transcript {
            // Surface whatever language we ended up using: the
            // explicit hint if one was given, otherwise fall back to
            // None and let the UI label it as "auto-detected".
            language: hint.map(|s| s.to_string()),
            segments,
        })
    }
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
