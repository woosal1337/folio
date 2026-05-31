//! Per-chunk language identification for code-switching recordings (GET-190).
//!
//! Whisper locks the decoded language to whatever it detected on the first
//! 30 s of a `full()` call and reuses that token for every later 30 s
//! window in the same call (OpenAI whisper#2009). A recording that
//! switches language mid-stream therefore gets the later language
//! transcribed as garbage — and the temperature-fallback re-decode smears
//! that garbage onto neighbouring windows, which is why "the English part
//! also degrades".
//!
//! The fix `local.rs` applies: transcribe in ≤ [`LID_WINDOW_SECONDS`]
//! windows, and for each window detect its language up front
//! (encoder-only via [`detect_language`], ~200–400 ms vs 2–6 s for a full
//! decode) and pass it explicitly to `full()`. Each `full()` call then
//! decodes exactly one window in one known language, so there is no
//! internal lock-in to leak across a language switch.
//!
//! The complementary half of the bug — `condition_on_previous_text`
//! seeding the next chunk with the previous chunk's (possibly hallucinated)
//! text — is already defused upstream via `set_no_context(true)` +
//! `set_n_max_text_ctx(0)` in `local.rs::transcribe`.

use whisper_rs::WhisperState;

/// Trust a fresh detection only at/above this top-language probability.
/// Below it, inherit the previous window's confirmed language so a noisy or
/// ambiguous window can't flip the transcript mid-stream.
pub const LID_CONFIDENCE_THRESHOLD: f32 = 0.80;

/// Windows shorter than this give unreliable language ID — inherit the
/// prior language rather than trust them.
pub const MIN_LID_SECONDS: f64 = 5.0;

/// Target per-window length handed to whisper. Kept just under whisper's
/// 30 s internal window so each `full()` call decodes exactly one window in
/// one language (no internal language lock-in across windows).
pub const LID_WINDOW_SECONDS: f64 = 28.0;

/// A per-window language detection.
#[derive(Debug, Clone, PartialEq)]
pub struct LangDetection {
    /// whisper language id (e.g. 0 = en).
    pub id: i32,
    /// ISO code (e.g. "en"); `None` if whisper returns an unknown id.
    pub code: Option<String>,
    /// Top language probability in `[0, 1]`.
    pub confidence: f32,
}

/// Detect the dominant language of a 16 kHz mono window using whisper's
/// encoder-only language head. Computes the mel for `samples_16k`, then
/// reads the language distribution. Returns `None` on whisper error.
///
/// Must be called with the same `WhisperState` that will run `full()`;
/// `full()` recomputes its own mel from the pcm slice, so this detection
/// pass does not perturb the subsequent transcription.
pub fn detect_language(
    state: &mut WhisperState,
    samples_16k: &[f32],
    threads: usize,
) -> Option<LangDetection> {
    // pcm_to_mel must run before lang_detect — lang_detect reads the mel
    // buffer this populates (calling it without is undefined per whisper).
    state.pcm_to_mel(samples_16k, threads).ok()?;
    let (id, probs) = state.lang_detect(0, threads).ok()?;
    let confidence = probs.iter().copied().fold(0.0_f32, f32::max);
    let code = whisper_rs::get_lang_str(id).map(|s| s.to_string());
    Some(LangDetection {
        id,
        code,
        confidence,
    })
}

/// Decide a window's language from its detection, duration, and the
/// previously confirmed language. Pure (no whisper) so the confidence +
/// min-duration + inherit-prior policy is unit-testable in isolation.
///
/// Returns `(language, confirmed)`:
/// - `language` — what to hand whisper's `set_language`. `None` only on the
///   very first window when nothing is confirmed yet *and* the detection is
///   untrusted; whisper then auto-detects internally.
/// - `confirmed` — the language to carry forward as the next window's
///   "prior". An untrusted window keeps the old prior rather than resetting
///   it, so one bad window can't drop the language for the rest of the run.
pub fn resolve_window_language(
    detection: Option<&LangDetection>,
    window_seconds: f64,
    prior: Option<&str>,
) -> (Option<String>, Option<String>) {
    let trusted = detection.and_then(|d| {
        let ok = window_seconds >= MIN_LID_SECONDS && d.confidence >= LID_CONFIDENCE_THRESHOLD;
        if ok {
            d.code.clone()
        } else {
            None
        }
    });
    match trusted {
        Some(code) => (Some(code.clone()), Some(code)),
        None => {
            let inherited = prior.map(|s| s.to_string());
            (inherited.clone(), inherited)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn det(code: &str, confidence: f32) -> LangDetection {
        LangDetection {
            id: 0,
            code: Some(code.to_string()),
            confidence,
        }
    }

    #[test]
    fn confident_long_window_is_trusted() {
        let d = det("tr", 0.93);
        let (lang, confirmed) = resolve_window_language(Some(&d), 20.0, Some("en"));
        assert_eq!(lang.as_deref(), Some("tr"));
        assert_eq!(confirmed.as_deref(), Some("tr"));
    }

    #[test]
    fn low_confidence_inherits_prior() {
        let d = det("tr", 0.55);
        let (lang, confirmed) = resolve_window_language(Some(&d), 20.0, Some("en"));
        // A shaky detection must not flip the language mid-stream.
        assert_eq!(lang.as_deref(), Some("en"));
        assert_eq!(confirmed.as_deref(), Some("en"));
    }

    #[test]
    fn short_window_inherits_prior_even_if_confident() {
        let d = det("tr", 0.99);
        let (lang, confirmed) = resolve_window_language(Some(&d), 3.0, Some("en"));
        assert_eq!(lang.as_deref(), Some("en"));
        assert_eq!(confirmed.as_deref(), Some("en"));
    }

    #[test]
    fn first_window_untrusted_yields_none() {
        let d = det("tr", 0.40);
        let (lang, confirmed) = resolve_window_language(Some(&d), 4.0, None);
        // Nothing confirmed yet → let whisper auto-detect internally.
        assert_eq!(lang, None);
        assert_eq!(confirmed, None);
    }

    #[test]
    fn no_detection_inherits_prior() {
        let (lang, confirmed) = resolve_window_language(None, 20.0, Some("de"));
        assert_eq!(lang.as_deref(), Some("de"));
        assert_eq!(confirmed.as_deref(), Some("de"));
    }
}
