//! Pure prompt + message builders shared by the agent runners (GET-184).
//!
//! Everything here is data-in/data-out — it reads files and core types and
//! returns strings — so it lives in the library rather than the Tauri
//! binary (thin-binary / fat-library rule). The `run_agent` command shim in
//! `attune-app` orchestrates these; the logic itself is testable here.

use std::path::Path;

use crate::transcription::SessionTranscript;

/// Hard cap on the transcript text handed to an agent, in bytes. Beyond
/// this the transcript is truncated on a char boundary (GET-175) before the
/// model sees it.
pub const TRANSCRIPT_CHAR_CAP: usize = 100_000;

/// Build the language trailer appended to every agent's system prompt.
/// Closes v2 roadmap finding R09 / implements 097.
///
/// `briefing_language` is the user's Settings choice:
///   * `"auto"` → mirror the meeting's language (legacy behaviour).
///   * any other BCP-47 tag → force that language regardless of the
///     transcript, including tool-call free-text fields (task titles,
///     memory content, autoname title/subtitle).
///
/// We do not auto-detect from the transcript ourselves — the model picks up
/// the meeting's dominant script from the user message that follows. The
/// trailer just states the rule.
pub fn language_aware_trailer(briefing_language: &str) -> String {
    let trimmed = briefing_language.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        return "\n\n\
LANGUAGE: Always reply in the same language as the meeting transcript, \
not the language of these instructions. If the transcript is mixed, \
default to the dominant language. For tool calls, write `title`, \
`content`, `notes`, and any other free-text fields in the meeting's \
language; structural fields (kind, key, status) stay in English."
            .to_string();
    }
    let name = language_name(trimmed);
    format!(
        "\n\n\
LANGUAGE: Always reply in {name} regardless of the language of the \
meeting transcript or these instructions. Translate any quoted snippets \
into {name} when surfacing them in your prose, but keep verbatim \
evidence snippets in their original language so they still match the \
transcript. For tool calls, write `title`, `content`, `notes`, and any \
other free-text fields in {name}; structural fields (kind, key, status) \
stay in English."
    )
}

/// Map a BCP-47 / ISO-639-1 tag to the English name the model knows best.
/// Unknown tags pass through verbatim so a niche tag like `cy` (Welsh)
/// still produces a sensible instruction rather than crashing.
fn language_name(tag: &str) -> String {
    match tag.to_ascii_lowercase().as_str() {
        "en" => "English".to_string(),
        "tr" => "Turkish".to_string(),
        "az" => "Azerbaijani".to_string(),
        "ru" => "Russian".to_string(),
        "de" => "German".to_string(),
        "es" => "Spanish".to_string(),
        "fr" => "French".to_string(),
        "it" => "Italian".to_string(),
        "pt" => "Portuguese".to_string(),
        "nl" => "Dutch".to_string(),
        "pl" => "Polish".to_string(),
        "ar" => "Arabic".to_string(),
        "ja" => "Japanese".to_string(),
        "zh" => "Chinese".to_string(),
        "ko" => "Korean".to_string(),
        "uk" => "Ukrainian".to_string(),
        "he" => "Hebrew".to_string(),
        "hi" => "Hindi".to_string(),
        other => other.to_string(),
    }
}

/// A short human-facing summary of what an agent run produced, used as the
/// run's `response` text for the task/memory extractors.
pub fn synth_summary(agent_id: &str, tasks: usize, memories: usize) -> String {
    match agent_id {
        "extract-tasks" if tasks == 0 => "No explicit action items found.".to_string(),
        "extract-tasks" => format!(
            "Created {tasks} task{} from this recording.",
            if tasks == 1 { "" } else { "s" }
        ),
        "extract-memories" if memories == 0 => "No new memories extracted.".to_string(),
        "extract-memories" => format!(
            "Captured {memories} memory{} from this recording.",
            if memories == 1 { "y" } else { "ies" }
        ),
        _ => format!("Agent run completed with {tasks} task(s), {memories} memor(y/ies)."),
    }
}

/// The session directory's leaf name, used as a human-readable recording
/// label for citations / provenance.
pub fn session_label_from_dir(session_dir: &Path) -> Option<String> {
    session_dir
        .file_name()
        .and_then(|os| os.to_str())
        .map(|s| s.to_string())
}

/// Render the transcript the way the agents read it: one chronological,
/// speaker-labelled dialogue ("You:" for the note-taker, "Speaker N:" for
/// each diarized participant — or the real name the user gave that voice,
/// from the session's speaker sidecar). See
/// `SessionTranscript::to_labeled_dialogue_named` — the shared formatter so
/// the summary, Q&A, and the editor agree on labels. No timestamps here;
/// the agent prompts don't cite moments.
pub fn flatten_transcript(session_dir: &Path, transcript: &SessionTranscript) -> String {
    let names = crate::diarization::SessionSpeakers::read(session_dir)
        .ok()
        .flatten()
        .map(|s| s.name_map())
        .unwrap_or_default();
    transcript.to_labeled_dialogue_named(false, &names)
}

/// Build the user message handed to an agent: the speaker-labelled
/// transcript (truncated on a char boundary past [`TRANSCRIPT_CHAR_CAP`]),
/// then the user's live notes, then their section outline when present.
pub fn build_user_message(
    transcript_text: &str,
    live_notes_md: Option<&str>,
    note_outline: Option<&str>,
) -> String {
    // Legend so the model reads the speaker labels right: the transcript
    // is one chronological dialogue, a line per turn, each prefixed with
    // its speaker. "You:" is the note-taker (their mic); "Speaker 1",
    // "Speaker 2", … are the other participants told apart by voice
    // (diarization). Attributing points to these labels is what makes the
    // summary precise about who said/owns what.
    const LEGEND: &str = "Meeting transcript — a chronological dialogue, one \
        line per speaker turn, each prefixed with the speaker. \"You:\" is \
        the note-taker (their own microphone). \"Speaker 1\", \"Speaker 2\", \
        … are the other participants, told apart by voice. \"Others:\" is \
        unattributed audio. Attribute points, decisions, and action items \
        to the right speaker by these labels.";
    let mut out = if transcript_text.len() <= TRANSCRIPT_CHAR_CAP {
        format!("{LEGEND}\n\n{}", transcript_text)
    } else {
        // Char-boundary truncation — a byte slice panics mid-codepoint on
        // multilingual transcripts (GET-175).
        let truncated = crate::text::truncate_on_char_boundary(transcript_text, TRANSCRIPT_CHAR_CAP);
        format!(
            "{LEGEND}\n\n(truncated to first {} characters; full transcript \
            was {} characters)\n\n{}",
            TRANSCRIPT_CHAR_CAP,
            transcript_text.len(),
            truncated,
        )
    };
    if let Some(notes) = live_notes_md {
        let notes = notes.trim();
        if !notes.is_empty() {
            out.push_str(
                "\n\n<user_live_notes>\n\
                These are the notes the user typed live during the meeting. \
                Treat them as high-signal: fold their action items / \
                decisions / questions into the matching sections without \
                duplicating what the transcript already covers.\n\n",
            );
            out.push_str(notes);
            out.push_str("\n</user_live_notes>");
        }
    }
    // GET-195: the user's own section headings become the note's spine.
    if let Some(outline) = note_outline {
        let outline = outline.trim();
        if !outline.is_empty() {
            out.push_str(
                "\n\n<user_section_outline>\n\
                The user sketched these section headings (and any notes under \
                them) in their live notes. Build the enhanced note around \
                EXACTLY these headings, in this order — flesh each out from \
                the transcript and the user's lines. Keep the headings \
                verbatim. Follow the OUTLINE MODE rule in your \
                instructions.\n\n",
            );
            out.push_str(outline);
            out.push_str("\n</user_section_outline>");
        }
    }
    out
}

/// Read a session's live notes (GET-145) and render them as the grouped
/// markdown the summary agent folds in. None when the session has no notes
/// or the file is missing/unreadable.
pub fn read_live_notes_markdown(session_dir: &Path) -> Option<String> {
    let bytes = std::fs::read(session_dir.join("live_notes.json")).ok()?;
    let lines: Vec<crate::live_notes::RawNoteLine> = serde_json::from_slice(&bytes).ok()?;
    let notes = crate::live_notes::parse_lines(&lines);
    if notes.is_empty() {
        return None;
    }
    Some(crate::live_notes::render_markdown(&notes))
}

/// Read the user-authored section outline from a session's live notes
/// (GET-195): the markdown headings the user typed + their seed lines.
/// `None` when the user wrote no headers — the summary then uses its
/// default four-section template.
pub fn read_note_outline(session_dir: &Path) -> Option<String> {
    let bytes = std::fs::read(session_dir.join("live_notes.json")).ok()?;
    let lines: Vec<crate::live_notes::RawNoteLine> = serde_json::from_slice(&bytes).ok()?;
    let outline = crate::live_notes::extract_outline(&lines);
    if outline.is_empty() {
        return None;
    }
    Some(crate::live_notes::render_outline_scaffold(&outline))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::{ChannelTranscript, SessionTranscript, TranscriptSegment};

    fn ch(channel: &str, texts: &[&str]) -> ChannelTranscript {
        ChannelTranscript {
            channel: channel.to_string(),
            language: None,
            segments: texts
                .iter()
                .enumerate()
                .map(|(i, t)| TranscriptSegment {
                    start_seconds: i as f64,
                    end_seconds: (i + 1) as f64,
                    text: t.to_string(),
                    speaker: None,
                    language: None,
                })
                .collect(),
        }
    }

    #[test]
    fn trailer_auto_keeps_legacy_meeting_language_rule() {
        for tag in ["auto", "Auto", " AUTO ", ""] {
            let t = language_aware_trailer(tag);
            assert!(
                t.contains("same language as the meeting transcript"),
                "tag={tag:?}"
            );
            assert!(!t.contains("regardless of the language"), "tag={tag:?}");
        }
    }

    #[test]
    fn trailer_known_tag_forces_named_language() {
        let t = language_aware_trailer("en");
        assert!(t.contains("Always reply in English"));
        assert!(t.contains("regardless of the language"));
        let t = language_aware_trailer("tr");
        assert!(t.contains("Always reply in Turkish"));
    }

    #[test]
    fn trailer_unknown_tag_passes_through() {
        let t = language_aware_trailer("cy");
        assert!(t.contains("Always reply in cy"));
    }

    #[test]
    fn trailer_evidence_snippet_rule_only_when_forcing_translation() {
        assert!(!language_aware_trailer("auto").contains("verbatim evidence snippets"));
        assert!(language_aware_trailer("en").contains("verbatim evidence snippets"));
    }

    #[test]
    fn flatten_includes_speaker_labels() {
        let t = SessionTranscript {
            channels: vec![
                ch("mic", &["Merhaba.", "Nasılsın?"]),
                ch("system", &["İyiyim, teşekkürler."]),
            ],
        };
        let text = flatten_transcript(std::path::Path::new("/nonexistent"), &t);
        assert!(text.contains("You: Merhaba."), "got: {text}");
        assert!(text.contains("Others: İyiyim, teşekkürler."), "got: {text}");
    }

    #[test]
    fn flatten_skips_empty_channels() {
        let t = SessionTranscript {
            channels: vec![ch("mic", &[]), ch("system", &["Single line"])],
        };
        let text = flatten_transcript(std::path::Path::new("/nonexistent"), &t);
        assert!(!text.contains("You:"));
        assert!(text.contains("Single line"));
    }

    #[test]
    fn user_message_truncates_oversized_input() {
        let huge = "x".repeat(TRANSCRIPT_CHAR_CAP * 2);
        let msg = build_user_message(&huge, None, None);
        assert!(msg.contains("truncated to first"));
        assert!(msg.len() < TRANSCRIPT_CHAR_CAP + 500);
    }

    #[test]
    fn user_message_does_not_panic_on_oversized_multilingual_input() {
        // GET-175: a byte-slice cut at the cap could land mid-codepoint on
        // multibyte text and panic. A Turkish transcript past the cap must
        // truncate cleanly.
        let huge = "Şu an ekranı mı kaydediyor? ".repeat(TRANSCRIPT_CHAR_CAP);
        assert!(huge.len() > TRANSCRIPT_CHAR_CAP);
        let msg = build_user_message(&huge, None, None);
        assert!(msg.contains("truncated to first"));
    }

    #[test]
    fn user_message_appends_live_notes_block_when_present() {
        let msg = build_user_message("hello", Some("## Action items\n\n- `0:05` ship"), None);
        assert!(msg.contains("<user_live_notes>"));
        assert!(msg.contains("## Action items"));
        assert!(msg.contains("ship"));
        let bare = build_user_message("hello", Some("   "), None);
        assert!(!bare.contains("<user_live_notes>"));
    }

    #[test]
    fn user_message_appends_section_outline_when_present() {
        // GET-195: the user's headings reach the agent as a scaffold block.
        let msg = build_user_message("hello", None, Some("## Risks\n- vendor lock-in"));
        assert!(msg.contains("<user_section_outline>"));
        assert!(msg.contains("## Risks"));
        assert!(msg.contains("vendor lock-in"));
        let bare = build_user_message("hello", None, None);
        assert!(!bare.contains("<user_section_outline>"));
    }
}
