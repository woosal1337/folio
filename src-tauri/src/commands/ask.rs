//! Per-note scoped chat — "Chat with this transcript" (GET-150).
//!
//! A conversation restricted to a single meeting's corpus: its
//! transcript, the notes the user typed live (GET-145), and any agent
//! runs (the summary, decisions, …). Distinct from the cross-library
//! Ask Attune, which spans everything. Because one meeting fits well
//! inside the context budget, this skips retrieval entirely and packs
//! the whole note into the system prompt, with transcript timestamps the
//! model is told to cite as `[mm:ss]` so the UI can make them seekable.

use std::path::Path;

use attune_core::llm::provider::LlmProvider;
use attune_core::llm::{ChatMessage, ChatRequest, ChatRole, KeyStore, OpenAiProvider, ProviderId};
use attune_core::transcription::SessionTranscript;
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::info;

use crate::app::AppState;

const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";
const TRANSCRIPT_CHAR_CAP: usize = 100_000;

/// One prior turn in the per-note conversation.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatTurn {
    /// "user" or "assistant".
    pub role: String,
    pub content: String,
}

/// The assistant's answer to a scoped question.
#[derive(Debug, Clone, Serialize)]
pub struct AskNoteAnswer {
    pub answer: String,
}

const SYSTEM_PROMPT: &str = "You are answering questions about ONE meeting. \
The context below — the transcript (with [mm:ss] timestamps), the user's \
live notes, and any generated summary — is the ONLY source you may use.\n\
\n\
Rules:\n\
  - Answer strictly from the provided context. If the answer is not in it, \
say \"That isn't covered in this meeting.\" Never invent content.\n\
  - When you reference a moment, cite its timestamp in square brackets like \
[12:34] using the transcript's timestamps. The app turns these into \
clickable jumps.\n\
  - Be concise and direct.";

#[tauri::command]
pub async fn ask_note(
    state: State<'_, AppState>,
    session_dir: String,
    question: String,
    history: Vec<ChatTurn>,
) -> Result<AskNoteAnswer, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    let dir = {
        let root = output_dir.clone();
        let target = std::path::PathBuf::from(&session_dir);
        tauri::async_runtime::spawn_blocking(move || {
            attune_core::paths::canonicalize_under(&root, &target).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("canonicalize task panicked: {e}"))??
    };

    // Assemble the note's corpus on a blocking thread (disk reads).
    let context = {
        let dir = dir.clone();
        tauri::async_runtime::spawn_blocking(move || build_note_context(&dir))
            .await
            .map_err(|e| format!("context build panicked: {e}"))?
    };
    if context.trim().is_empty() {
        return Err("this note has no transcript or notes to chat about yet".into());
    }

    let api_key = tauri::async_runtime::spawn_blocking(move || KeyStore::get(ProviderId::OpenAi))
        .await
        .map_err(|e| format!("keystore lookup panicked: {e}"))?
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            "no OpenAI API key configured. Open Settings → AI and paste your key.".to_string()
        })?;

    let system_prompt = format!("{SYSTEM_PROMPT}\n\n<note_context>\n{context}\n</note_context>");
    let mut messages: Vec<ChatMessage> = Vec::with_capacity(history.len() + 1);
    for turn in history {
        let role = match turn.role.as_str() {
            "assistant" => ChatRole::Assistant,
            _ => ChatRole::User,
        };
        messages.push(ChatMessage {
            role,
            content: turn.content,
            tool_calls: None,
            tool_call_id: None,
        });
    }
    messages.push(ChatMessage {
        role: ChatRole::User,
        content: question,
        tool_calls: None,
        tool_call_id: None,
    });

    let provider = OpenAiProvider::new(api_key);
    let response = provider
        .chat(ChatRequest {
            model: DEFAULT_OPENAI_MODEL.to_string(),
            system_prompt,
            messages,
            temperature: Some(0.2),
            max_tokens: None,
            tools: None,
        })
        .await
        .map_err(|e| e.to_string())?;

    info!(session = %dir.display(), "answered scoped note question");
    Ok(AskNoteAnswer {
        answer: response.text,
    })
}

/// Build the scoped context block: timestamped transcript + live notes +
/// agent-run outputs. Capped to keep within the model's context budget.
fn build_note_context(dir: &Path) -> String {
    let mut out = String::new();

    if let Ok(transcript) = SessionTranscript::read_json(&dir.join("transcript.json")) {
        let text = flatten_with_timestamps(&transcript);
        if !text.is_empty() {
            out.push_str("## Transcript\n");
            if text.len() > TRANSCRIPT_CHAR_CAP {
                out.push_str(&text[..TRANSCRIPT_CHAR_CAP]);
                out.push_str("\n[transcript truncated]");
            } else {
                out.push_str(&text);
            }
            out.push_str("\n\n");
        }
    }

    if let Ok(bytes) = std::fs::read(dir.join("live-notes.md")) {
        if let Ok(md) = String::from_utf8(bytes) {
            if !md.trim().is_empty() {
                out.push_str("## Notes the user typed live\n");
                out.push_str(md.trim());
                out.push_str("\n\n");
            }
        }
    }

    if let Ok(runs) = attune_core::llm::AgentRunStore::list(dir) {
        for run in runs {
            if run.response.trim().is_empty() {
                continue;
            }
            out.push_str(&format!("## {} (generated)\n", run.agent_name));
            out.push_str(run.response.trim());
            out.push_str("\n\n");
        }
    }

    out.trim().to_string()
}

fn flatten_with_timestamps(transcript: &SessionTranscript) -> String {
    let mut out = String::new();
    for channel in &transcript.channels {
        if channel.segments.is_empty() {
            continue;
        }
        let label = match channel.channel.as_str() {
            "mic" => "[You]",
            "system" => "[Others]",
            other => other,
        };
        out.push_str(label);
        out.push('\n');
        for seg in &channel.segments {
            let text = seg.text.trim();
            if text.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "[{}] {}\n",
                format_timestamp(seg.start_seconds),
                text
            ));
        }
        out.push('\n');
    }
    out.trim().to_string()
}

fn format_timestamp(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_are_mm_ss_or_h_mm_ss() {
        assert_eq!(format_timestamp(0.0), "00:00");
        assert_eq!(format_timestamp(42.0), "00:42");
        assert_eq!(format_timestamp(125.0), "02:05");
        assert_eq!(format_timestamp(3725.0), "1:02:05");
    }

    #[test]
    fn flatten_labels_channels_and_prefixes_timestamps() {
        use attune_core::transcription::{ChannelTranscript, TranscriptSegment};
        let t = SessionTranscript {
            channels: vec![ChannelTranscript {
                channel: "mic".into(),
                language: None,
                segments: vec![TranscriptSegment {
                    start_seconds: 65.0,
                    end_seconds: 67.0,
                    text: "pricing decision".into(),
                }],
            }],
        };
        let text = flatten_with_timestamps(&t);
        assert!(text.contains("[You]"));
        assert!(text.contains("[01:05] pricing decision"));
    }
}
