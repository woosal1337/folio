use crate::transcription::TranscriptSegment;

const WHISPER_ARTIFACT_PHRASES: &[&str] = &[
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
    "altyazı m k",
    "altyazi m k",
    "altyazı mk",
    "altyazı by mk",
    "yorumlarınızıza abone olmayı unutmayın",
    "abone olmayı unutmayın",
    "abone olun",
    "kanalımıza abone olun",
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
    "sottotitoli creati dalla comunità amara org",
    "sottotitoli e revisione a cura di amara org",
    "sottotitoli e revisione al canale di amara org",
    "sottotitoli e revisione a cura di qtss",
    "sottotitoli a cura di qtss",
    "subtítulos realizados por la comunidad de amara org",
    "subtitulado por la comunidad de amara org",
    "subtítulos por la comunidad de amara org",
    "subtítulos creados por la comunidad de amara org",
    "subtítulos en español de amara org",
    "subtítulos hechos por la comunidad de amara org",
    "más información www alimmenta com",
    "legendas pela comunidade amara org",
    "legendas pela comunidade de amara org",
    "legendas pela comunidade do amara org",
    "transcrição e legendas pela comunidade de amara org",
    "ondertitels ingediend door de amara org gemeenschap",
    "ondertiteld door de amara org gemeenschap",
    "ondertiteling door de amara org gemeenschap",
    "napisy stworzone przez społeczność amara org",
    "napisy wykonane przez społeczność amara org",
    "tłumaczenie i napisy stworzone przez społeczność amara org",
    "tłumaczenie stworzone przez społeczność amara org",
    "субтитры сделал dimatorzok",
    "редактор субтитров а синецкая корректор а егорова",
    "продолжение следует",
    "字幕由amara org社区提供",
    "字幕由amara org社區提供",
    "由amara org 社群提供的字幕",
    "小編字幕由amara org社區提供",
    "中文字幕志愿者 杨茜茜",
    "中文字幕 yk",
];

const WHISPER_ARTIFACT_MARKERS: &[&str] = &[
    "amara org",
    "soustitreur",
    "mooji org",
    "dimatorzok",
    "ming pao",
    "ming pao canada",
    "ming pao toronto",
    "zdf für funk",
    "untertitel im auftrag des zdf",
    "copyright wdr",
    "altyazı m k",
    "altyazi m k",
    "transcribed by castingwords",
    "transcribed by https otter ai",
    "www mooji org",
    "www multi moto eu",
];

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

const REPETITION_LOOP_MIN_RUN: usize = 3;

pub fn dedupe_repetitions(
    segments: Vec<TranscriptSegment>,
) -> (Vec<TranscriptSegment>, Vec<String>) {
    if segments.len() < REPETITION_LOOP_MIN_RUN {
        return (segments, Vec::new());
    }

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

            continue;
        }
        for offset in 0..len {
            kept.push(segments[start + offset].clone());
        }
    }
    (kept, dropped)
}

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

        assert!(!is_whisper_hallucination(
            "Thank you for the detailed explanation of the architecture"
        ));
        assert!(!is_whisper_hallucination(
            "We had a great barbecue last weekend and I want to thank you"
        ));

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
        let input = vec![seg("Yes."), seg("Yes."), seg("Then we move on.")];
        let (kept, dropped) = dedupe_repetitions(input);
        assert_eq!(kept.len(), 3);
        assert!(dropped.is_empty());
    }

    #[test]
    fn dedupe_drops_runs_of_three_or_more() {
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
