//! Post-decode filter for Whisper's "Thank you." / "you" / "Thanks for
//! watching." artifacts.
//!
//! Even with `no_context=true` and `no_speech_thold=0.8`, whisper.cpp
//! still occasionally emits a single tiny segment on chunks dominated
//! by silence or background music. The training data is full of
//! YouTube captions ending in "Thank you for watching." and the model
//! falls back to those when nothing else fits.
//!
//! The 2026-05 benchmark on RunPod (see
//! `~/Documents/GitHub/obsidian.md/projects/attune/research/stt-benchmark-report.md`)
//! confirmed this happens on every Whisper variant (large-v3, large-v3-turbo,
//! faster-whisper, CrisperWhisper) and *only* on Whisper-family models —
//! CTC/TDT decoders (Parakeet, Canary) emit nothing on the same input.
//! So the fix lives here in the Whisper-specific path.
//!
//! We strip these by exact-match against a small curated phrase list
//! after normalizing case, punctuation, and whitespace. Substring
//! matches are intentionally not used: a real meeting line like
//! "Thank you for joining today" stays.

use crate::transcription::TranscriptSegment;

/// Canonical Whisper artifact phrases, post-normalization (lowercase,
/// stripped of ASCII punctuation, single-spaced).
///
/// Sources: empirical output from the 2026-05 RunPod bake-off, plus
/// the long-running list on github.com/openai/whisper/discussions.
const WHISPER_ARTIFACT_PHRASES: &[&str] = &[
    "you",
    "thank you",
    "thanks for watching",
    "thank you for watching",
    "thanks for watching everyone",
    "thanks for watching this video",
    "please subscribe",
    "subscribe to my channel",
    "like and subscribe",
    "bye",
    "bye bye",
    "okay",
    "ok",
    "music",
    "applause",
    "silence",
];

/// Returns true if `text`, after normalization, exactly matches one of
/// the known Whisper artifact phrases. Empty strings count as
/// hallucinations too.
pub fn is_whisper_hallucination(text: &str) -> bool {
    let normalized = normalize_for_match(text);
    if normalized.is_empty() {
        return true;
    }
    WHISPER_ARTIFACT_PHRASES.contains(&normalized.as_str())
}

/// Strip Whisper artifact segments out of `segments`. Returns the
/// kept segments and the number that were dropped, so callers can
/// log it.
pub fn filter_segments(segments: Vec<TranscriptSegment>) -> (Vec<TranscriptSegment>, usize) {
    let original = segments.len();
    let kept: Vec<TranscriptSegment> = segments
        .into_iter()
        .filter(|seg| !is_whisper_hallucination(&seg.text))
        .collect();
    let dropped = original - kept.len();
    (kept, dropped)
}

fn normalize_for_match(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_was_space = true;
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            for c in ch.to_lowercase() {
                out.push(c);
            }
            last_was_space = false;
        } else if !last_was_space {
            out.push(' ');
            last_was_space = true;
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(text: &str) -> TranscriptSegment {
        TranscriptSegment {
            start_seconds: 0.0,
            end_seconds: 1.0,
            text: text.to_string(),
        }
    }

    #[test]
    fn normalizes_case_punctuation_and_whitespace() {
        assert_eq!(normalize_for_match("Thank you."), "thank you");
        assert_eq!(normalize_for_match(" Thank   you !  "), "thank you");
        assert_eq!(normalize_for_match("YOU"), "you");
        assert_eq!(normalize_for_match("..."), "");
    }

    #[test]
    fn classic_whisper_silence_phrases_are_hallucinations() {
        assert!(is_whisper_hallucination("Thank you."));
        assert!(is_whisper_hallucination("Thanks for watching!"));
        assert!(is_whisper_hallucination(" you "));
        assert!(is_whisper_hallucination("."));
        assert!(is_whisper_hallucination(""));
        assert!(is_whisper_hallucination("Please subscribe."));
        assert!(is_whisper_hallucination("Music"));
    }

    #[test]
    fn real_sentences_are_not_hallucinations() {
        assert!(!is_whisper_hallucination("Merhaba dünya"));
        assert!(!is_whisper_hallucination("Thank you for joining today"));
        assert!(!is_whisper_hallucination(
            "Bizim ekip tamamen yazılım geçmişli birileridir"
        ));
        assert!(!is_whisper_hallucination("Yes."));
        assert!(!is_whisper_hallucination("No."));
        assert!(!is_whisper_hallucination(
            "It is one of the most popular tourist destinations"
        ));
    }

    #[test]
    fn filter_drops_hallucinations_and_reports_count() {
        let segments = vec![
            seg("El elemleri koşturan kişinin bir arkitektür"),
            seg("Thank you."),
            seg("Çok desteklerim ben ama şey yani"),
            seg("you"),
            seg("Thanks for watching!"),
            seg("Bizim ekip aslında tamamen yazılım"),
        ];
        let (kept, dropped) = filter_segments(segments);
        assert_eq!(dropped, 3);
        assert_eq!(kept.len(), 3);
        assert!(kept.iter().all(|s| !s.text.is_empty()));
    }

    #[test]
    fn filter_passes_empty_input_through() {
        let (kept, dropped) = filter_segments(vec![]);
        assert!(kept.is_empty());
        assert_eq!(dropped, 0);
    }

    #[test]
    fn filter_keeps_everything_when_nothing_matches() {
        let segments = vec![
            seg("Merhaba"),
            seg("Bu bir test cümlesidir"),
            seg("Hello world"),
        ];
        let (kept, dropped) = filter_segments(segments);
        assert_eq!(dropped, 0);
        assert_eq!(kept.len(), 3);
    }
}
