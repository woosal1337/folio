use std::path::PathBuf;

use attune_core::transcription::local::LocalWhisperTranscriber;
use attune_core::transcription::Transcriber;

fn model_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ATTUNE_WHISPER_MODEL") {
        let pb = PathBuf::from(p);
        return pb.is_file().then_some(pb);
    }

    let home = std::env::var_os("HOME")?;
    let pb =
        PathBuf::from(home).join("Library/Application Support/Attune/models/ggml-large-v3.bin");
    pb.is_file().then_some(pb)
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../e2e/fixtures/audio")
}

fn wav(name: &str) -> Option<PathBuf> {
    let p = fixtures_dir().join(name);
    p.is_file().then_some(p)
}

fn transcript_text(t: &attune_core::transcription::Transcript) -> String {
    t.segments
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn transcribe_fixture(
    file: &str,
    language_hint: Option<&str>,
) -> Option<attune_core::transcription::Transcript> {
    let Some(model) = model_path() else {
        eprintln!(
            "SKIP: whisper model not found (set ATTUNE_WHISPER_MODEL or download via the app)"
        );
        return None;
    };
    let Some(audio) = wav(file) else {
        eprintln!("SKIP: fixture {file} not found — run `bun run e2e:fixtures`");
        return None;
    };
    let t = LocalWhisperTranscriber::new(model)
        .with_threads(4)
        .transcribe(&audio, language_hint)
        .expect("transcription must not error");
    Some(t)
}

#[test]
fn english_business_clip_transcribes_with_expected_keywords() {
    let Some(t) = transcribe_fixture("en-business-1min.wav", Some("en")) else {
        return;
    };
    let text = transcript_text(&t);
    assert!(!text.trim().is_empty(), "transcript should not be empty");

    let keywords = ["launch", "marketing", "migration", "referral"];
    let hits = keywords.iter().filter(|k| text.contains(**k)).count();
    assert!(
        hits >= 2,
        "expected >=2 of {keywords:?} in transcript, got {hits}.\nTranscript: {text}"
    );
}

#[test]
#[ignore = "slow: large-v3 ~15s; run with --ignored"]
fn english_clinical_clip_transcribes() {
    let Some(t) = transcribe_fixture("en-clinical-consult.wav", Some("en")) else {
        return;
    };
    let text = transcript_text(&t);
    let keywords = [
        "sleep",
        "medication",
        "side effects",
        "appetite",
        "nightmares",
    ];
    let hits = keywords.iter().filter(|k| text.contains(**k)).count();
    assert!(hits >= 1, "expected clinical vocab in: {text}");
}

#[test]
#[ignore = "slow: large-v3 ~15s; run with --ignored"]
fn turkish_clip_detects_language_and_transcribes() {
    let Some(t) = transcribe_fixture("tr-meeting.wav", None) else {
        return;
    };
    let text = transcript_text(&t);
    assert!(
        !text.trim().is_empty(),
        "turkish transcript should not be empty"
    );

    if let Some(lang) = &t.language {
        assert_eq!(
            lang.to_lowercase(),
            "tr",
            "expected Turkish detection, got {lang}"
        );
    }
}

#[test]
#[ignore = "slow: large-v3 ~15s; run with --ignored"]
fn german_clip_transcribes() {
    let Some(t) = transcribe_fixture("de-standup.wav", Some("de")) else {
        return;
    };
    assert!(!transcript_text(&t).trim().is_empty());
}

#[test]
#[ignore = "slow: large-v3 ~15s; run with --ignored"]
fn french_clip_transcribes() {
    let Some(t) = transcribe_fixture("fr-product-pitch.wav", Some("fr")) else {
        return;
    };
    let text = transcript_text(&t);
    assert!(!text.trim().is_empty());

    let keywords = ["mac", "transcription", "cloud"];
    assert!(
        keywords.iter().any(|k| text.contains(*k)),
        "expected one of {keywords:?} in: {text}"
    );
}

#[test]
#[ignore = "slow: large-v3 ~15s; run with --ignored"]
fn spanish_clip_transcribes() {
    let Some(t) = transcribe_fixture("es-clinical-followup.wav", Some("es")) else {
        return;
    };
    assert!(!transcript_text(&t).trim().is_empty());
}

#[test]
#[ignore = "slow: large-v3 ~15s; run with --ignored"]
fn action_items_clip_carries_assignees() {
    let Some(t) = transcribe_fixture("en-action-items.wav", Some("en")) else {
        return;
    };
    let text = transcript_text(&t);

    let verbs = [
        "draft",
        "confirm",
        "embargo",
        "wednesday",
        "thursday",
        "friday",
    ];
    let hits = verbs.iter().filter(|k| text.contains(**k)).count();
    assert!(hits >= 2, "expected action vocab in: {text}");
}

#[test]
fn fixtures_and_model_presence_is_reported() {
    let model = model_path();
    let fixture = wav("en-business-1min.wav");
    eprintln!(
        "transcription fixtures: model={} fixture={}",
        model
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "ABSENT".into()),
        fixture
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "ABSENT".into()),
    );

    assert!(
        fixtures_dir().exists() || std::env::var("CI").is_ok(),
        "e2e/fixtures/audio dir not found at {}",
        fixtures_dir().display()
    );
}
