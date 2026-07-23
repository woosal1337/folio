use std::path::Path;

use folio_core::llm::provider::LlmProvider;
use folio_core::llm::{ChatMessage, ChatRequest, ChatRole, KeyStore, OpenAiProvider, ProviderId};
use folio_core::storage::{scan_recordings, TaskStatus, TaskStore};
use folio_core::transcription::SessionTranscript;
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::info;

use crate::app::AppState;

const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";
const TRANSCRIPT_CHAR_CAP: usize = 100_000;

#[derive(Debug, Clone, Deserialize)]
pub struct ChatTurn {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AskNoteAnswer {
    pub answer: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageNote {
    pub notes_total: usize,

    pub notes_read: usize,

    pub capped: bool,

    pub date_oldest: Option<String>,

    pub date_newest: Option<String>,

    pub memories: usize,

    pub tasks: usize,
}

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
            folio_core::paths::canonicalize_under(&root, &target).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("canonicalize task panicked: {e}"))??
    };

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

fn build_note_context(dir: &Path) -> String {
    let mut out = String::new();

    if let Ok(transcript) = SessionTranscript::read_json(&dir.join("transcript.json")) {
        let text = flatten_with_timestamps(dir, &transcript);
        if !text.is_empty() {
            out.push_str("## Transcript\n");
            if text.len() > TRANSCRIPT_CHAR_CAP {
                out.push_str(folio_core::text::truncate_on_char_boundary(
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

    if let Ok(runs) = folio_core::llm::AgentRunStore::list(dir) {
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

fn flatten_with_timestamps(session_dir: &Path, transcript: &SessionTranscript) -> String {
    let names = folio_core::diarization::SessionSpeakers::read(session_dir)
        .ok()
        .flatten()
        .map(|s| s.name_map())
        .unwrap_or_default();
    transcript.to_labeled_dialogue_named(true, &names)
}

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

fn build_folder_context(
    output_dir: &Path,
    tasks_path: &Path,
    memory_store: &folio_core::memory::MemoryStore,
    query: &str,
    folder_name: &str,
) -> (String, CoverageNote) {
    let mut out = String::new();

    let tasks = TaskStore::new(tasks_path.to_path_buf()).list();
    let open: Vec<_> = tasks
        .iter()
        .filter(|t| t.status != TaskStatus::Done)
        .collect();
    let tasks_count = open.len();

    let mut recordings = scan_recordings(output_dir);
    recordings.sort_by_key(|r| std::cmp::Reverse(r.created_at));
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
        let summary = folio_core::llm::AgentRunStore::list(dir)
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

fn build_library_context(
    output_dir: &Path,
    tasks_path: &Path,
    memory_store: &folio_core::memory::MemoryStore,
    query: &str,
) -> (String, CoverageNote) {
    let mut out = String::new();

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

    use folio_core::llm::retrieval;

    let query_tokens_owned = retrieval::tokenize_query(query);
    let query_tokens: Vec<&str> = query_tokens_owned.iter().map(String::as_str).collect();
    let today = chrono::Utc::now();

    let mut recordings = scan_recordings(output_dir);
    let notes_total = recordings.len();

    let mut scored: Vec<(f32, &folio_core::storage::RecordingSummary, String)> = recordings
        .iter()
        .filter_map(|r| {
            let dir = Path::new(&r.session_dir);
            let summary = folio_core::llm::AgentRunStore::list(dir)
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

    recordings.sort_by_key(|r| std::cmp::Reverse(r.created_at));
    let capped = included >= LIBRARY_RECENT_NOTES && notes_total > included;

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
        use folio_core::transcription::{ChannelTranscript, TranscriptSegment};
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

        assert!(text.contains("[1:05] You: pricing decision"), "got: {text}");
    }
}
