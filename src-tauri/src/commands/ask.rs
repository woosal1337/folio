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
use attune_core::storage::{scan_recordings, TaskStatus, TaskStore};
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

/// Coverage metadata returned alongside the cross-library answer
/// (GET-193). Lets the frontend render a "what was searched" panel
/// so the user knows the answer's scope and completeness.
#[derive(Debug, Clone, Serialize)]
pub struct CoverageNote {
    /// Total recordings found in the library.
    pub notes_total: usize,
    /// Recordings whose summaries were actually read by the LLM.
    pub notes_read: usize,
    /// True when the read count hit LIBRARY_RECENT_NOTES and more
    /// summarised notes exist — the answer may be incomplete.
    pub capped: bool,
    /// `created_at` of the oldest note that was read. ISO-ish string
    /// from the session directory name (e.g. "2026-05-01").
    pub date_oldest: Option<String>,
    /// `created_at` of the newest note that was read.
    pub date_newest: Option<String>,
    /// Number of memories injected from the relevant-memory search.
    pub memories: usize,
    /// Number of open tasks included in the context.
    pub tasks: usize,
}

/// The cross-library Ask Attune answer, augmented with coverage metadata.
#[derive(Debug, Clone, Serialize)]
pub struct AskLibraryAnswer {
    pub answer: String,
    pub coverage: CoverageNote,
}

const SYSTEM_PROMPT: &str = "You are answering questions about ONE meeting. \
The context below — the transcript (with [mm:ss] timestamps), the user's \
live notes, and any generated summary — is the ONLY source you may use.\n\
\n\
The transcript is a multi-speaker dialogue: each line is \"[mm:ss] \
Speaker: text\". \"You:\" is the person asking (the note-taker); \"Speaker \
1\", \"Speaker 2\", … are the other participants, told apart by voice. Use \
these labels when you attribute statements, and do not invent real names \
for the numbered speakers.\n\
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
        let text = flatten_with_timestamps(dir, &transcript);
        if !text.is_empty() {
            out.push_str("## Transcript\n");
            if text.len() > TRANSCRIPT_CHAR_CAP {
                // Char-boundary truncation — a byte slice panics mid-
                // codepoint on multilingual transcripts (GET-175).
                out.push_str(attune_core::text::truncate_on_char_boundary(
                    &text,
                    TRANSCRIPT_CHAR_CAP,
                ));
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

/// Render the transcript as one chronological, speaker-labelled dialogue
/// with `[mm:ss]` prefixes so the Q&A model can cite moments. Labels are
/// "You:" (note-taker) and "Speaker N:" (each diarized participant), or the
/// real name the user gave that voice (session speaker sidecar). See
/// `SessionTranscript::to_labeled_dialogue_named` — shared with the agents.
fn flatten_with_timestamps(session_dir: &Path, transcript: &SessionTranscript) -> String {
    let names = attune_core::diarization::SessionSpeakers::read(session_dir)
        .ok()
        .flatten()
        .map(|s| s.name_map())
        .unwrap_or_default();
    transcript.to_labeled_dialogue_named(true, &names)
}

// ====================================================================
// Cross-library chat (GET-152)
// ====================================================================

/// How many recent recordings' summaries to fold into the library
/// context. Bounded so the context stays inside the model budget.
const LIBRARY_RECENT_NOTES: usize = 8;

const LIBRARY_SYSTEM_PROMPT: &str = "You are the user's meeting brain. You \
answer across their whole library — open action items, recent meeting \
summaries, and remembered facts, all provided below. Use only that \
context.\n\
\n\
Rules:\n\
  - Ground every claim in the context. If something isn't there, say you \
don't have it rather than inventing it.\n\
  - When you reference a meeting, name it so the user can find it.\n\
  - Be concise and well-structured; use short headers or bullets when it \
helps the user act.";

#[tauri::command]
pub async fn ask_library(
    state: State<'_, AppState>,
    question: String,
    history: Vec<ChatTurn>,
    model: Option<String>,
) -> Result<AskLibraryAnswer, String> {
    let (output_dir, tasks_path) = {
        let s = state.settings.lock();
        (s.output_dir.clone(), s.tasks_path.clone())
    };
    let memory_store = state.memory_store()?;
    let query = question.clone();

    let (context, coverage) = tauri::async_runtime::spawn_blocking(move || {
        build_library_context(&output_dir, &tasks_path, &memory_store, &query)
    })
    .await
    .map_err(|e| format!("library context panicked: {e}"))?;

    if context.trim().is_empty() {
        return Err("your library is empty — record a meeting first".into());
    }

    let api_key = tauri::async_runtime::spawn_blocking(move || KeyStore::get(ProviderId::OpenAi))
        .await
        .map_err(|e| format!("keystore lookup panicked: {e}"))?
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            "no OpenAI API key configured. Open Settings → AI and paste your key.".to_string()
        })?;

    let system_prompt =
        format!("{LIBRARY_SYSTEM_PROMPT}\n\n<library_context>\n{context}\n</library_context>");
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
            model: model.unwrap_or_else(|| DEFAULT_OPENAI_MODEL.to_string()),
            system_prompt,
            messages,
            temperature: Some(0.3),
            max_tokens: None,
            tools: None,
        })
        .await
        .map_err(|e| e.to_string())?;

    info!("answered cross-library question");
    Ok(AskLibraryAnswer {
        answer: response.text,
        coverage,
    })
}

/// Ask a question scoped to a specific folder (GET-205).
///
/// Behaves like `ask_library` but only includes notes in `folder_name`.
/// Returns the same `AskLibraryAnswer` (answer + coverage metadata).
#[tauri::command]
pub async fn ask_folder(
    state: State<'_, AppState>,
    folder_name: String,
    question: String,
    history: Vec<ChatTurn>,
    model: Option<String>,
) -> Result<AskLibraryAnswer, String> {
    let (output_dir, tasks_path) = {
        let s = state.settings.lock();
        (s.output_dir.clone(), s.tasks_path.clone())
    };
    let memory_store = state.memory_store()?;
    let query = question.clone();
    let folder = folder_name.clone();

    let (context, coverage) = tauri::async_runtime::spawn_blocking(move || {
        build_folder_context(&output_dir, &tasks_path, &memory_store, &query, &folder)
    })
    .await
    .map_err(|e| format!("folder context panicked: {e}"))?;

    if context.trim().is_empty() {
        return Err(format!(
            "no summarised notes found in folder \"{folder_name}\" — run the Summarize agent on notes first"
        ));
    }

    let api_key = tauri::async_runtime::spawn_blocking(move || KeyStore::get(ProviderId::OpenAi))
        .await
        .map_err(|e| format!("keystore lookup panicked: {e}"))?
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            "no OpenAI API key configured. Open Settings → AI and paste your key.".to_string()
        })?;

    let system_prompt = format!(
        "{}\n\n<folder_context folder=\"{folder_name}\">\n{context}\n</folder_context>",
        LIBRARY_SYSTEM_PROMPT
    );
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
            model: model.unwrap_or_else(|| DEFAULT_OPENAI_MODEL.to_string()),
            system_prompt,
            messages,
            temperature: Some(0.3),
            max_tokens: None,
            tools: None,
        })
        .await
        .map_err(|e| e.to_string())?;

    info!(folder = %folder_name, "answered folder-scoped question");
    Ok(AskLibraryAnswer {
        answer: response.text,
        coverage,
    })
}

/// Build context from only the notes in `folder_name`.
fn build_folder_context(
    output_dir: &Path,
    tasks_path: &Path,
    memory_store: &attune_core::memory::MemoryStore,
    query: &str,
    folder_name: &str,
) -> (String, CoverageNote) {
    let mut out = String::new();

    // Filter tasks to those from the folder's notes.
    let tasks = TaskStore::new(tasks_path.to_path_buf()).list();
    let open: Vec<_> = tasks
        .iter()
        .filter(|t| t.status != TaskStatus::Done)
        .collect();
    let tasks_count = open.len();

    // Folder-filtered summaries.
    let mut recordings = scan_recordings(output_dir);
    recordings.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let in_folder: Vec<_> = recordings
        .iter()
        .filter(|r| r.folder.as_deref() == Some(folder_name))
        .collect();
    let notes_total = in_folder.len();
    let mut included = 0;
    let mut date_oldest: Option<String> = None;
    let mut date_newest: Option<String> = None;

    for r in &in_folder {
        let dir = Path::new(&r.session_dir);
        let summary = attune_core::llm::AgentRunStore::list(dir)
            .ok()
            .and_then(|runs| runs.into_iter().find(|run| run.agent_id == "summarize"))
            .map(|run| run.response);
        let Some(summary) = summary else { continue };
        if summary.trim().is_empty() {
            continue;
        }
        let title = r.suggested_title.as_deref().unwrap_or(&r.label);
        out.push_str(&format!("## Note: {title}\n"));
        out.push_str(summary.trim());
        out.push_str("\n\n");
        let date_str = r.created_at.map(|dt| dt.format("%Y-%m-%d").to_string());
        if date_newest.is_none() {
            date_newest = date_str.clone();
        }
        date_oldest = date_str;
        included += 1;
    }
    let capped = included >= LIBRARY_RECENT_NOTES && notes_total > included;

    let mut memories_count = 0;
    if let Ok(memories) = memory_store.search(query, None, &[], 6) {
        memories_count = memories.len();
        if !memories.is_empty() {
            out.push_str("## Remembered facts\n");
            for m in memories {
                let key = m.key.as_deref().unwrap_or("");
                out.push_str(&format!("- {key}: {}\n", m.content));
            }
        }
    }

    let coverage = CoverageNote {
        notes_total,
        notes_read: included,
        capped,
        date_oldest,
        date_newest,
        memories: memories_count,
        tasks: tasks_count,
    };
    (out.trim().to_string(), coverage)
}

/// Assemble the cross-library context: open tasks, recent meeting
/// summaries, and the memories most relevant to the question.
/// Returns the context string and coverage metadata (GET-193).
fn build_library_context(
    output_dir: &Path,
    tasks_path: &Path,
    memory_store: &attune_core::memory::MemoryStore,
    query: &str,
) -> (String, CoverageNote) {
    let mut out = String::new();

    // Open action items.
    let tasks = TaskStore::new(tasks_path.to_path_buf()).list();
    let open: Vec<_> = tasks
        .iter()
        .filter(|t| t.status != TaskStatus::Done)
        .collect();
    let tasks_count = open.len();
    if !open.is_empty() {
        out.push_str("## Open action items\n");
        for t in &open {
            let owner = t
                .owner
                .as_deref()
                .map(|o| format!(" ({o})"))
                .unwrap_or_default();
            let due = t
                .due
                .as_deref()
                .map(|d| format!(" — due {d}"))
                .unwrap_or_default();
            let src = t
                .source_session_label
                .as_deref()
                .map(|s| format!(" [from {s}]"))
                .unwrap_or_default();
            out.push_str(&format!("- {}{owner}{due}{src}\n", t.title));
        }
        out.push('\n');
    }

    // Iterative retrieval (GET-203): relevance-ranked notes-over-transcripts.
    //
    // Tier 1 — collect all summaries, score against query tokens, sort by
    //   combined (relevance × recency) score, take top LIBRARY_RECENT_NOTES.
    // Tier 2 — for the top TRANSCRIPT_EXCERPT_MAX notes with a high relevance
    //   score, append a verbatim transcript excerpt so verbatim questions are
    //   answered with quotable evidence.
    use attune_core::llm::retrieval;

    let query_tokens_owned = retrieval::tokenize_query(query);
    let query_tokens: Vec<&str> = query_tokens_owned.iter().map(String::as_str).collect();
    let today = chrono::Utc::now();

    let mut recordings = scan_recordings(output_dir);
    let notes_total = recordings.len();

    // Collect summaries + scores.
    let mut scored: Vec<(f32, &attune_core::storage::RecordingSummary, String)> = recordings
        .iter()
        .filter_map(|r| {
            let dir = Path::new(&r.session_dir);
            let summary = attune_core::llm::AgentRunStore::list(dir)
                .ok()
                .and_then(|runs| runs.into_iter().find(|run| run.agent_id == "summarize"))
                .map(|run| run.response)?;
            if summary.trim().is_empty() {
                return None;
            }
            let days_ago = r
                .created_at
                .map(|dt| (today - dt).num_days() as f64)
                .unwrap_or(180.0);
            let rel = retrieval::relevance_score(&summary, &query_tokens);
            let score = retrieval::combined_score(rel, days_ago);
            Some((score, r, summary))
        })
        .collect();

    // Sort descending by score.
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(LIBRARY_RECENT_NOTES);

    const TRANSCRIPT_EXCERPT_MAX: usize = 2;
    const HIGH_RELEVANCE_THRESHOLD: f32 = 0.5;

    let mut included = 0;
    let mut date_oldest: Option<String> = None;
    let mut date_newest: Option<String> = None;

    for (score, r, summary) in &scored {
        let title = r.suggested_title.as_deref().unwrap_or(&r.label);
        out.push_str(&format!("## Meeting: {title}\n"));
        out.push_str(summary.trim());

        // Tier 2: pull transcript excerpt for high-relevance notes.
        if included < TRANSCRIPT_EXCERPT_MAX
            && *score >= HIGH_RELEVANCE_THRESHOLD
            && !query_tokens.is_empty()
        {
            let dir = Path::new(&r.session_dir);
            if let Some(excerpt) = retrieval::transcript_excerpt(dir, &query_tokens, 400) {
                out.push_str("\n\n*Transcript excerpt:* \"");
                out.push_str(&excerpt);
                out.push('"');
            }
        }
        out.push_str("\n\n");

        let date_str = r.created_at.map(|dt| dt.format("%Y-%m-%d").to_string());
        if date_newest.is_none() {
            date_newest = date_str.clone();
        }
        date_oldest = date_str;
        included += 1;
    }

    // Re-sort recordings for capped check (we need a stable ordering).
    recordings.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let capped = included >= LIBRARY_RECENT_NOTES && notes_total > included;

    // Relevant memories.
    let mut memories_count = 0;
    if let Ok(memories) = memory_store.search(query, None, &[], 8) {
        memories_count = memories.len();
        if !memories.is_empty() {
            out.push_str("## Remembered facts\n");
            for m in memories {
                let key = m.key.as_deref().unwrap_or("");
                out.push_str(&format!("- {key}: {}\n", m.content));
            }
            out.push('\n');
        }
    }

    let coverage = CoverageNote {
        notes_total,
        notes_read: included,
        capped,
        date_oldest,
        date_newest,
        memories: memories_count,
        tasks: tasks_count,
    };
    (out.trim().to_string(), coverage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_labels_speakers_and_prefixes_timestamps() {
        use attune_core::transcription::{ChannelTranscript, TranscriptSegment};
        let t = SessionTranscript {
            channels: vec![ChannelTranscript {
                channel: "mic".into(),
                language: None,
                segments: vec![TranscriptSegment {
                    start_seconds: 65.0,
                    end_seconds: 67.0,
                    text: "pricing decision".into(),
                    speaker: None,
                    language: None,
                }],
            }],
        };
        let text = flatten_with_timestamps(std::path::Path::new("/nonexistent"), &t);
        // The mic channel is the note-taker, labelled "You", with an
        // [m:ss] prefix the Q&A model can cite.
        assert!(text.contains("[1:05] You: pricing decision"), "got: {text}");
    }
}
