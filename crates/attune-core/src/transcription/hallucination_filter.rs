//! Post-decode filter for Whisper's training-data artifact hallucinations.
//!
//! Two failure modes from the same root cause:
//!
//! 1. **Silence-fill artifacts**: even with `no_context=true` and
//!    `no_speech_thold=0.8`, whisper.cpp still occasionally emits a
//!    tiny segment on chunks dominated by silence or music. The model
//!    falls back to high-frequency training samples — "Thank you.",
//!    "you", "Thanks for watching."
//!
//! 2. **Subtitle-credit hallucinations**: OpenAI trained Whisper on
//!    680k hours of YouTube audio paired with community-contributed
//!    subtitles. Credits like "Subtitles by the Amara.org community",
//!    "Untertitel im Auftrag des ZDF", "Sottotitoli e revisione a
//!    cura di QTSS", and (in Turkish) "Altyazı M.K." were never
//!    stripped from the training set. The model memorises them as
//!    things that "must appear" near silence and emits them on quiet
//!    chunks regardless of the actual audio. See
//!    [openai/whisper#928](https://github.com/openai/whisper/discussions/928),
//!    [openai/whisper#1873](https://github.com/openai/whisper/discussions/1873),
//!    [openai/whisper#2412](https://github.com/openai/whisper/discussions/2412)
//!    for the long-running multilingual catalog this list is derived
//!    from.
//!
//! The Attune 2026-05 RunPod bake-off confirmed CTC/TDT decoders
//! (Parakeet, Canary) emit nothing on the same input, so this is a
//! Whisper-family problem only. The fix is post-decode filtering, not
//! a model swap.
//!
//! ## Matching strategy
//!
//! Two passes, both after normalization (lowercase, NFKC implicit via
//! `char::to_lowercase`, strip everything that is not alphanumeric,
//! collapse whitespace).
//!
//! - **Exact phrase match** for short generic English artifacts and
//!   the well-formed multilingual subtitle credits.
//! - **Substring marker match** for the families with too many
//!   variants to enumerate (Amara.org has ~30 wordings; ZDF/WDR have
//!   per-year copyright lines). The markers are unmistakable
//!   (domain names, broadcaster IDs, translator handles) and will not
//!   appear in legitimate meeting speech.
//!
//! Real sentences containing the artifact phrases as substrings (e.g.
//! "Thank you for joining today") stay intact because the *exact*
//! match is on the normalized full segment, not on substrings.

use crate::transcription::TranscriptSegment;

/// Canonical Whisper artifact phrases, post-normalization.
///
/// All entries must already be in normalized form (lowercase, no
/// punctuation, single-spaced) so we can compare against the
/// normalized segment text directly.
const WHISPER_ARTIFACT_PHRASES: &[&str] = &[
    // --- Bare English silence artifacts ---
    "you",
    "thank you",
    "thanks for watching",
    "thank you for watching",
    "thanks for watching everyone",
    "thanks for watching this video",
    "thank you so much for watching",
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
    "transcribed by castingwords",
    // --- Turkish (the user's primary language, see github discussion #2412) ---
    "altyazı m k",
    "altyazi m k",
    "altyazı mk",
    "altyazı by mk",
    "yorumlarınızıza abone olmayı unutmayın",
    "abone olmayı unutmayın",
    "abone olun",
    "kanalımıza abone olun",
    // --- German (ZDF / WDR / Amara subtitle credits, discussion #928) ---
    "untertitel der amara org community",
    "untertitelung aufgrund der amara org community",
    "untertitel von stephanie geiges",
    "untertitel im auftrag des zdf für funk 2017",
    "untertitel im auftrag des zdf 2017",
    "untertitel im auftrag des zdf 2018",
    "untertitel im auftrag des zdf 2020",
    "untertitel im auftrag des zdf 2021",
    "untertitelung im auftrag des zdf 2021",
    "copyright wdr 2019",
    "copyright wdr 2020",
    "copyright wdr 2021",
    "swr 2020",
    "swr 2021",
    // --- French (Amara + SousTitreur + ST'501) ---
    "sous titres réalisés par la communauté d amara org",
    "sous titres réalisés para la communauté d amara org",
    "sous titres fait par sous titres par amara org",
    "sous titres par amara org",
    "sous titres par la communauté d amara org",
    "sous titres réalisés pour la communauté d amara org",
    "sous titrage st 501",
    "par soustitreur com",
    "merci d avoir regardé cette vidéo",
    "merci d avoir regardé la vidéo",
    "merci d avoir regardé",
    "je vous remercie de vous abonner",
    "j espère que vous avez apprécié la vidéo",
    // --- Italian (QTSS + Amara) ---
    "sottotitoli creati dalla comunità amara org",
    "sottotitoli e revisione a cura di amara org",
    "sottotitoli e revisione al canale di amara org",
    "sottotitoli e revisione a cura di qtss",
    "sottotitoli a cura di qtss",
    // --- Spanish ---
    "subtítulos realizados por la comunidad de amara org",
    "subtitulado por la comunidad de amara org",
    "subtítulos por la comunidad de amara org",
    "subtítulos creados por la comunidad de amara org",
    "subtítulos en español de amara org",
    "subtítulos hechos por la comunidad de amara org",
    "más información www alimmenta com",
    // --- Portuguese ---
    "legendas pela comunidade amara org",
    "legendas pela comunidade de amara org",
    "legendas pela comunidade do amara org",
    "transcrição e legendas pela comunidade de amara org",
    // --- Dutch ---
    "ondertitels ingediend door de amara org gemeenschap",
    "ondertiteld door de amara org gemeenschap",
    "ondertiteling door de amara org gemeenschap",
    // --- Polish ---
    "napisy stworzone przez społeczność amara org",
    "napisy wykonane przez społeczność amara org",
    "tłumaczenie i napisy stworzone przez społeczność amara org",
    "tłumaczenie stworzone przez społeczność amara org",
    // --- Russian (DimaTorzok signature + Sinetskaya/Egorova editorial credit) ---
    "субтитры сделал dimatorzok",
    "редактор субтитров а синецкая корректор а егорова",
    "продолжение следует",
    // --- Chinese (multiple Amara variants + Ming Pao + volunteer credits) ---
    "字幕由amara org社区提供",
    "字幕由amara org社區提供",
    "由amara org 社群提供的字幕",
    "小編字幕由amara org社區提供",
    "中文字幕志愿者 杨茜茜",
    "中文字幕 yk",
];

/// Substring markers for hallucination families with too many wordings
/// to enumerate. If any marker appears in the normalized segment text,
/// the whole segment is treated as a hallucination.
///
/// These are chosen to be unmistakable (domain names, broadcaster
/// short codes, translator handles, dataset signature initials) so
/// real meeting speech will not collide with them.
const WHISPER_ARTIFACT_MARKERS: &[&str] = &[
    "amara org",   // any "Amara.org" subtitle credit, ~30 languages
    "soustitreur", // French SousTitreur.com signature
    "mooji org",   // Mooji subtitle leakage (en, es)
    "dimatorzok",  // Russian subtitle handle
    "ming pao",    // Hong Kong newspaper subtitle artifact
    "ming pao canada",
    "ming pao toronto",
    "zdf für funk",                  // German ZDF/funk credit (any year)
    "untertitel im auftrag des zdf", // catches any year variant
    "copyright wdr",                 // catches any year variant
    "altyazı m k",                   // Turkish "Altyazı M.K." across all spacings
    "altyazi m k",
    "transcribed by castingwords",
    "transcribed by https otter ai",
    "www mooji org",
    "www multi moto eu",
];

/// Returns true if `text`, after normalization, matches one of the
/// known Whisper artifact phrases (exact match) or contains one of
/// the known marker substrings.
///
/// Empty strings count as hallucinations too: there is no reason for
/// the model to emit an empty segment, and downstream UI does not
/// want them.
pub fn is_whisper_hallucination(text: &str) -> bool {
    let normalized = normalize_for_match(text);
    if normalized.is_empty() {
        return true;
    }
    if WHISPER_ARTIFACT_PHRASES.contains(&normalized.as_str()) {
        return true;
    }
    WHISPER_ARTIFACT_MARKERS
        .iter()
        .any(|m| normalized.contains(m))
}

/// Minimum run length that counts as a hallucination loop. Two
/// identical consecutive segments can happen legitimately ("Yes."
/// "Yes.") so we wait for the third before dropping anything.
const REPETITION_LOOP_MIN_RUN: usize = 3;

/// Drop runs of `REPETITION_LOOP_MIN_RUN` or more consecutive
/// segments whose normalized text is identical. Returns the kept
/// segments and the dropped texts (deduplicated to one entry per
/// run so the log line stays readable).
///
/// Whisper falls into contextual hallucination loops on silent
/// chunks: in the 2026-05-26-11-47-54 mic recording it emitted
/// "I'm going to ask you to take your own distance from there." 14
/// times at exact 2-second cadence while the user was silent and
/// listening to background audio. The text was novel (not in the
/// curated artifact catalog) but the loop structure is unmistakable.
/// Two-then-stop happens in real meetings ("Yes." / "Yes."); three or
/// more identical segments in a row is the signature of a stuck
/// decoder.
pub fn dedupe_repetitions(
    segments: Vec<TranscriptSegment>,
) -> (Vec<TranscriptSegment>, Vec<String>) {
    if segments.len() < REPETITION_LOOP_MIN_RUN {
        return (segments, Vec::new());
    }
    // Pass 1: compute run boundaries (start_idx, length) so we can
    // decide which slots to drop without recomputing comparisons.
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < segments.len() {
        let key = normalize_for_match(&segments[i].text);
        let mut j = i + 1;
        while j < segments.len() && normalize_for_match(&segments[j].text) == key {
            j += 1;
        }
        runs.push((i, j - i));
        i = j;
    }
    let mut kept = Vec::with_capacity(segments.len());
    let mut dropped = Vec::new();
    for (start, len) in runs {
        if len >= REPETITION_LOOP_MIN_RUN {
            dropped.push(segments[start].text.clone());
            // Drop every member of the run, including the first.
            // Once whisper enters the loop the "first" copy is just
            // as much a hallucination as the rest — keeping it
            // would leave a confusing fragment in the transcript.
            continue;
        }
        for offset in 0..len {
            kept.push(segments[start + offset].clone());
        }
    }
    (kept, dropped)
}

/// Strip Whisper artifact segments out of `segments`. Returns the
/// kept segments alongside the text of every segment that was
/// dropped, so callers can log which artifact triggered the filter.
/// Visibility is the point: without the dropped text, a "0 segments
/// kept, 2 dropped" log line gives no way to tell whether the filter
/// caught real hallucinations or accidentally killed real speech.
pub fn filter_segments(segments: Vec<TranscriptSegment>) -> (Vec<TranscriptSegment>, Vec<String>) {
    let mut kept = Vec::with_capacity(segments.len());
    let mut dropped = Vec::new();
    for seg in segments {
        if is_whisper_hallucination(&seg.text) {
            dropped.push(seg.text);
        } else {
            kept.push(seg);
        }
    }
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
            speaker: None,
            language: None,
        }
    }

    #[test]
    fn normalizes_case_punctuation_and_whitespace() {
        assert_eq!(normalize_for_match("Thank you."), "thank you");
        assert_eq!(normalize_for_match(" Thank   you !  "), "thank you");
        assert_eq!(normalize_for_match("YOU"), "you");
        assert_eq!(normalize_for_match("..."), "");
        assert_eq!(normalize_for_match("Altyazı M.K."), "altyazı m k");
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
    fn turkish_subtitle_credit_is_hallucination() {
        assert!(is_whisper_hallucination("Altyazı M.K."));
        assert!(is_whisper_hallucination("Altyazi M.K."));
        assert!(is_whisper_hallucination("altyazı m.k."));
        assert!(is_whisper_hallucination(" Altyazı M.K. "));
        assert!(is_whisper_hallucination(
            "Yorumlarınızıza abone olmayı unutmayın."
        ));
        assert!(is_whisper_hallucination("Abone olmayı unutmayın!"));
    }

    #[test]
    fn amara_org_in_any_language_is_hallucination() {
        // All these are real samples from github.com/openai/whisper/discussions/928
        assert!(is_whisper_hallucination(
            "Sous-titres réalisés par la communauté d'Amara.org"
        ));
        assert!(is_whisper_hallucination(
            "Untertitel der Amara.org-Community"
        ));
        assert!(is_whisper_hallucination(
            "Sottotitoli creati dalla comunità Amara.org"
        ));
        assert!(is_whisper_hallucination(
            "Subtítulos por la comunidad de Amara.org"
        ));
        assert!(is_whisper_hallucination(
            "Legendas pela comunidade Amara.org"
        ));
        assert!(is_whisper_hallucination(
            "Ondertitels ingediend door de Amara.org gemeenschap"
        ));
        assert!(is_whisper_hallucination(
            "Napisy stworzone przez społeczność Amara.org"
        ));
    }

    #[test]
    fn german_zdf_wdr_credits_are_hallucinations() {
        assert!(is_whisper_hallucination(
            "Untertitel im Auftrag des ZDF, 2017"
        ));
        assert!(is_whisper_hallucination(
            "Untertitel im Auftrag des ZDF für funk, 2017"
        ));
        assert!(is_whisper_hallucination("Copyright WDR 2021"));
    }

    #[test]
    fn italian_qtss_is_hallucination() {
        assert!(is_whisper_hallucination(
            "Sottotitoli e revisione a cura di QTSS"
        ));
        assert!(is_whisper_hallucination("Sottotitoli a cura di QTSS."));
    }

    #[test]
    fn french_soustitreur_is_hallucination() {
        assert!(is_whisper_hallucination("❤️ par SousTitreur.com"));
        assert!(is_whisper_hallucination("— Sous-titrage ST'501 —"));
    }

    #[test]
    fn russian_dimatorzok_is_hallucination() {
        assert!(is_whisper_hallucination("Субтитры сделал DimaTorzok"));
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
        // Sentence-level match must NOT trigger on substring of a long
        // legitimate sentence that happens to include an artifact phrase.
        assert!(!is_whisper_hallucination(
            "Thank you for the detailed explanation of the architecture"
        ));
        assert!(!is_whisper_hallucination(
            "We had a great barbecue last weekend and I want to thank you"
        ));
        // Real Turkish meeting content from the user's 2026-05-22 recording.
        assert!(!is_whisper_hallucination(
            "Bu Cloudedir, Giminal'dir. Bunların agent modlarını veya bu hani asistan modları var ya"
        ));
        assert!(!is_whisper_hallucination(
            "Onun haricinde şeyi sormuş olayım. Sizin kendi adresiniz projeniz yasada var mıydı?"
        ));
    }

    #[test]
    fn filter_drops_hallucinations_and_returns_their_text() {
        let segments = vec![
            seg("El elemleri koşturan kişinin bir arkitektür"),
            seg("Thank you."),
            seg("Çok desteklerim ben ama şey yani"),
            seg("you"),
            seg("Altyazı M.K."),
            seg("Thanks for watching!"),
            seg("Bizim ekip aslında tamamen yazılım"),
            seg("Sous-titres réalisés par la communauté d'Amara.org"),
            seg("Bu Cloudedir, Giminal'dir."),
        ];
        let (kept, dropped) = filter_segments(segments);
        assert_eq!(dropped.len(), 5);
        assert_eq!(kept.len(), 4);
        assert!(kept.iter().all(|s| !is_whisper_hallucination(&s.text)));
        assert!(dropped.contains(&"Altyazı M.K.".to_string()));
        assert!(dropped.contains(&"Thank you.".to_string()));
        assert!(dropped.contains(&"you".to_string()));
    }

    fn seg_at(text: &str, start: f64, end: f64) -> TranscriptSegment {
        TranscriptSegment {
            start_seconds: start,
            end_seconds: end,
            text: text.to_string(),
            speaker: None,
            language: None,
        }
    }

    #[test]
    fn dedupe_passes_through_segments_with_no_repetition() {
        let input = vec![seg("first"), seg("second"), seg("third"), seg("fourth")];
        let (kept, dropped) = dedupe_repetitions(input.clone());
        assert_eq!(kept.len(), 4);
        assert!(dropped.is_empty());
    }

    #[test]
    fn dedupe_keeps_pairs_of_identical_segments() {
        // Real meetings: "Yes." / "Yes." or "Right." / "Right." pairs
        // are legitimate confirmations and must survive.
        let input = vec![seg("Yes."), seg("Yes."), seg("Then we move on.")];
        let (kept, dropped) = dedupe_repetitions(input);
        assert_eq!(kept.len(), 3);
        assert!(dropped.is_empty());
    }

    #[test]
    fn dedupe_drops_runs_of_three_or_more() {
        // The 2026-05-26-11-47-54 mic hallucination pattern: 14
        // copies of the same phrase. Three is enough to trigger the
        // filter so we catch it early.
        let input = vec![
            seg_at("clean speech", 0.0, 5.0),
            seg_at(
                "I'm going to ask you to take your own distance from there.",
                43.22,
                45.22,
            ),
            seg_at(
                "I'm going to ask you to take your own distance from there.",
                45.22,
                47.22,
            ),
            seg_at(
                "I'm going to ask you to take your own distance from there.",
                47.22,
                49.22,
            ),
            seg_at("real speech after silence", 60.0, 62.0),
        ];
        let (kept, dropped) = dedupe_repetitions(input);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].text, "clean speech");
        assert_eq!(kept[1].text, "real speech after silence");
        assert_eq!(dropped.len(), 1);
        assert!(dropped[0].contains("take your own distance"));
    }

    #[test]
    fn dedupe_treats_punctuation_and_case_differences_as_same() {
        // Whisper sometimes varies trailing punctuation within a loop
        // ("hi" vs "hi." vs "Hi"). Normalization collapses these so
        // the run-length detector still fires.
        let input = vec![seg("hi"), seg("Hi."), seg("hi!"), seg("then more")];
        let (kept, dropped) = dedupe_repetitions(input);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].text, "then more");
        assert_eq!(dropped.len(), 1);
    }

    #[test]
    fn dedupe_handles_back_to_back_distinct_loops() {
        let input = vec![
            seg("loop one"),
            seg("loop one"),
            seg("loop one"),
            seg("loop two"),
            seg("loop two"),
            seg("loop two"),
            seg("loop two"),
            seg("kept after both"),
        ];
        let (kept, dropped) = dedupe_repetitions(input);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].text, "kept after both");
        assert_eq!(dropped.len(), 2);
    }

    #[test]
    fn filter_passes_empty_input_through() {
        let (kept, dropped) = filter_segments(vec![]);
        assert!(kept.is_empty());
        assert!(dropped.is_empty());
    }

    #[test]
    fn filter_keeps_everything_when_nothing_matches() {
        let segments = vec![
            seg("Merhaba"),
            seg("Bu bir test cümlesidir"),
            seg("Hello world"),
        ];
        let (kept, dropped) = filter_segments(segments);
        assert!(dropped.is_empty());
        assert_eq!(kept.len(), 3);
    }
}
