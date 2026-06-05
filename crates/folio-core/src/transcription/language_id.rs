use whisper_rs::WhisperState;

pub const LID_CONFIDENCE_THRESHOLD: f32 = 0.80;

pub const MIN_LID_SECONDS: f64 = 5.0;

pub const LID_WINDOW_SECONDS: f64 = 28.0;

#[derive(Debug, Clone, PartialEq)]
pub struct LangDetection {
    pub id: i32,

    pub code: Option<String>,

    pub confidence: f32,
}

pub fn detect_language(
    state: &mut WhisperState,
    samples_16k: &[f32],
    threads: usize,
) -> Option<LangDetection> {
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
