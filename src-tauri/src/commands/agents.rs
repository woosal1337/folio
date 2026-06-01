//! Tauri commands for running agents against a recording's transcript.
//!
//! Each `run_agent` call:
//!   1. Loads the recording's transcript from disk.
//!   2. Pulls the always-inject memory profile (identity + prefs +
//!      active projects + pinned memories) and prepends it to the
//!      agent's system prompt so every agent reads "what's true
//!      about this user" before reading the transcript.
//!   3. Builds a ChatRequest with the agent's tools attached.
//!      `extract-tasks` gets `create_task`; `extract-memories` gets
//!      `remember`; every agent gets `search_memory`.
//!   4. Calls OpenAI; on tool calls, dispatches them synchronously
//!      (MemoryStore + TaskStore writes), appends results, loops up
//!      to `MAX_TOOL_ITERATIONS`.
//!   5. After the loop, any memories the model created get their
//!      embeddings computed in parallel and upserted into the vec
//!      index (best-effort).
//!   6. Persists the final assistant text under
//!      `<session_dir>/agent_runs/<agent>.json`.

use std::path::PathBuf;

use attune_core::llm::agent_tools;
use attune_core::llm::agents;
use attune_core::llm::prompt;
use attune_core::llm::provider::LlmProvider;
use attune_core::llm::{
    Agent, AgentRun, AgentRunStore, ChatMessage, ChatRequest, ChatRole, KeyStore, OpenAiProvider,
    ProviderId,
};
use attune_core::memory::{EmbeddingClient, MemoryStore};
use attune_core::transcription::SessionTranscript;
use chrono::Utc;
use tauri::State;
use tracing::{debug, info, warn};

use crate::app::AppState;

const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";
const MAX_TOOL_ITERATIONS: usize = 5;

#[tauri::command]
pub fn list_agents() -> Vec<Agent> {
    debug!("list_agents");
    agents::defaults()
}

#[tauri::command]
pub async fn list_agent_runs(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<Vec<AgentRun>, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<AgentRun>, String> {
        let path = attune_core::paths::canonicalize_under(&output_dir, &session_dir)
            .map_err(|e| e.to_string())?;
        AgentRunStore::list(&path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("list_agent_runs task panicked: {e}"))?
}

#[tauri::command]
pub async fn delete_agent_run(
    state: State<'_, AppState>,
    session_dir: PathBuf,
    agent_id: String,
) -> Result<(), String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let path = attune_core::paths::canonicalize_under(&output_dir, &session_dir)
            .map_err(|e| e.to_string())?;
        AgentRunStore::delete(&path, &agent_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("delete_agent_run task panicked: {e}"))?
}

#[tauri::command]
pub async fn run_agent(
    state: State<'_, AppState>,
    session_dir: PathBuf,
    agent_id: String,
) -> Result<AgentRun, String> {
    let agent = agents::by_id(&agent_id).ok_or_else(|| format!("unknown agent id: {agent_id}"))?;

    let output_dir = state.settings.lock().output_dir.clone();
    let session_dir = {
        let target = session_dir.clone();
        let root = output_dir.clone();
        tauri::async_runtime::spawn_blocking(move || {
            attune_core::paths::canonicalize_under(&root, &target).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("canonicalize task panicked: {e}"))??
    };

    let transcript_path = session_dir.join("transcript.json");
    let session_dir_for_read = session_dir.clone();
    let transcript = tauri::async_runtime::spawn_blocking(move || {
        SessionTranscript::read_json(&session_dir_for_read.join("transcript.json"))
    })
    .await
    .map_err(|e| format!("transcript read task panicked: {e}"))?
    .map_err(|e| {
        format!(
            "could not read transcript at {}: {e}",
            transcript_path.display()
        )
    })?;

    let transcript_text = prompt::flatten_transcript(&session_dir, &transcript);
    if transcript_text.trim().is_empty() {
        return Err("transcript is empty — there is nothing for the agent to read".to_string());
    }
    // GET-147: the summary folds in the notes the user typed live during
    // the call so action items / decisions they captured seed the
    // structured note instead of being lost. Other agents read the
    // transcript only.
    let live_notes_md = if matches!(agent.id.as_str(), "summarize" | "write-followup-email") {
        let dir = session_dir.clone();
        tauri::async_runtime::spawn_blocking(move || prompt::read_live_notes_markdown(&dir))
            .await
            .unwrap_or(None)
    } else {
        None
    };
    // GET-195: when summarizing, the headings the user typed in their notes
    // become the note's section spine (the agent fleshes each out), instead
    // of the fixed template. Only the summary uses this.
    let note_outline = if agent.id == "summarize" {
        let dir = session_dir.clone();
        tauri::async_runtime::spawn_blocking(move || prompt::read_note_outline(&dir))
            .await
            .unwrap_or(None)
    } else {
        None
    };
    let user_message = prompt::build_user_message(
        &transcript_text,
        live_notes_md.as_deref(),
        note_outline.as_deref(),
    );

    // Snapshot paths + briefing language from settings (cheap, won't
    // block agent run). The lock is dropped before any IPC.
    let (tasks_path, briefing_language) = {
        let s = state.settings.lock();
        (s.tasks_path.clone(), s.briefing_language.clone())
    };
    // Shared MemoryStore from AppState — single SQLite connection
    // reused across this whole agent run (preamble + every tool
    // dispatch + post-run embedding). v2 finding R14.
    let memory_store = state.memory_store()?;

    // Build the "what's true about the user" preamble before any
    // network call. This runs on a blocking thread because MemoryStore
    // touches SQLite.
    let memory_preamble = {
        let store = memory_store.clone();
        tauri::async_runtime::spawn_blocking(move || -> Option<String> {
            let memories = store.always_inject_set(5).ok()?;
            if memories.is_empty() {
                return None;
            }
            let mut out = String::from("<user_memory>\n");
            for m in &memories {
                let key = m.key.as_deref().unwrap_or("");
                let pin = if m.pinned { "📌 " } else { "" };
                out.push_str(&format!("- {pin}{}: {}\n", key, m.content));
            }
            out.push_str("</user_memory>");
            Some(out)
        })
        .await
        .unwrap_or(None)
    };

    let provider_id = ProviderId::OpenAi;
    let api_key = tauri::async_runtime::spawn_blocking(move || KeyStore::get(provider_id))
        .await
        .map_err(|e| format!("keystore lookup panicked: {e}"))?
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            "no OpenAI API key configured. Open Settings → AI and paste your key.".to_string()
        })?;
    let provider = OpenAiProvider::new(api_key.clone());
    let model = DEFAULT_OPENAI_MODEL.to_string();

    let tools = agent_tools::tools_for_agent(&agent.id);
    let session_label = prompt::session_label_from_dir(&session_dir);

    // Compose system prompt:
    //   1. Memory preamble (if any) — background facts about the user
    //   2. The agent's own prompt — task instructions
    //   3. Language trailer — keeps output in the meeting's language
    //
    // The preamble is ABOVE the agent prompt so the model treats it as
    // background context, and the language rule is BELOW so it's the
    // last thing the model reads before responding (the strongest
    // position in a system prompt for behavioural overrides).
    let base = match memory_preamble {
        Some(preamble) => format!("{preamble}\n\n{}", agent.system_prompt),
        None => agent.system_prompt.clone(),
    };
    let system_prompt = format!(
        "{base}{}",
        prompt::language_aware_trailer(&briefing_language)
    );

    let mut messages: Vec<ChatMessage> = vec![ChatMessage {
        role: ChatRole::User,
        content: user_message,
        tool_calls: None,
        tool_call_id: None,
    }];

    let mut total_prompt_tokens: u32 = 0;
    let mut total_completion_tokens: u32 = 0;
    let mut final_text: String = String::new();
    let mut tasks_created: usize = 0;
    let mut memories_created: Vec<String> = Vec::new();

    info!(
        agent = %agent.id,
        provider = provider_id.as_str(),
        model = %model,
        transcript_chars = transcript_text.len(),
        tools_attached = tools.is_some(),
        "running agent",
    );

    for iteration in 0..MAX_TOOL_ITERATIONS {
        let request = ChatRequest {
            model: model.clone(),
            system_prompt: system_prompt.clone(),
            messages: messages.clone(),
            temperature: Some(0.2),
            max_tokens: None,
            tools: tools.clone(),
        };
        let response = provider.chat(request).await.map_err(|e| e.to_string())?;
        if let Some(p) = response.prompt_tokens {
            total_prompt_tokens = total_prompt_tokens.saturating_add(p);
        }
        if let Some(c) = response.completion_tokens {
            total_completion_tokens = total_completion_tokens.saturating_add(c);
        }

        if response.tool_calls.is_empty() {
            final_text = response.text;
            break;
        }

        debug!(
            iteration = iteration,
            calls = response.tool_calls.len(),
            "dispatching tool calls"
        );
        messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: response.text.clone(),
            tool_calls: Some(response.tool_calls.clone()),
            tool_call_id: None,
        });
        for call in &response.tool_calls {
            let result = agent_tools::dispatch_tool_call(
                call,
                &tasks_path,
                memory_store.clone(),
                session_dir.to_string_lossy().as_ref(),
                session_label.as_deref(),
                Some(&transcript),
            );
            match call.name.as_str() {
                "create_task" if result.success => {
                    tasks_created = tasks_created.saturating_add(1);
                }
                "remember" if result.success => {
                    if let Some(id) = &result.id {
                        memories_created.push(id.clone());
                    }
                }
                _ => {}
            }
            messages.push(ChatMessage {
                role: ChatRole::Tool,
                content: serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()),
                tool_calls: None,
                tool_call_id: Some(call.id.clone()),
            });
        }

        if iteration + 1 == MAX_TOOL_ITERATIONS {
            warn!(
                agent = %agent.id,
                "hit MAX_TOOL_ITERATIONS, stopping tool-dispatch loop"
            );
            final_text = format!("Stopped after {} tool-call rounds.", MAX_TOOL_ITERATIONS);
        }
    }

    if final_text.trim().is_empty() && tools.is_some() {
        final_text = prompt::synth_summary(&agent.id, tasks_created, memories_created.len());
    }

    // Best-effort embeddings for any memories the agent created.
    // Failures here are non-fatal: the memory still lives in the FTS5
    // index, search just falls back to BM25 for those rows.
    if !memories_created.is_empty() {
        embed_new_memories(&api_key, memory_store.clone(), &memories_created).await;
    }

    let run = AgentRun {
        agent_id: agent.id.clone(),
        agent_name: agent.name.clone(),
        provider: provider_id,
        model,
        response: final_text,
        prompt_tokens: if total_prompt_tokens == 0 {
            None
        } else {
            Some(total_prompt_tokens)
        },
        completion_tokens: if total_completion_tokens == 0 {
            None
        } else {
            Some(total_completion_tokens)
        },
        finished_at: Utc::now(),
    };

    let save_dir = session_dir.clone();
    let save_run = run.clone();
    tauri::async_runtime::spawn_blocking(move || AgentRunStore::save(&save_dir, &save_run))
        .await
        .map_err(|e| format!("agent run save task panicked: {e}"))?
        .map_err(|e| e.to_string())?;

    info!(
        agent = %run.agent_id,
        prompt_tokens = ?run.prompt_tokens,
        completion_tokens = ?run.completion_tokens,
        tasks_created,
        memories_created = memories_created.len(),
        "agent run complete",
    );
    Ok(run)
}

/// After the agent run finishes, fetch embeddings for the memories
/// it just created and upsert them into the vec index. Done outside
/// the tool loop because embedding is a network call and we don't
/// want to block tool dispatch on it. Errors are logged, not
/// surfaced — search still works via BM25.
async fn embed_new_memories(api_key: &str, store: std::sync::Arc<MemoryStore>, ids: &[String]) {
    let client = EmbeddingClient::new(api_key);
    for id in ids {
        let id_owned = id.clone();
        let store_for_get = store.clone();
        let memory = tauri::async_runtime::spawn_blocking(move || {
            store_for_get.get(&id_owned).ok().flatten()
        })
        .await
        .ok()
        .flatten();
        let Some(memory) = memory else { continue };
        let embedding = match client.embed(&memory.content).await {
            Ok(v) => v,
            Err(e) => {
                warn!(id = %memory.id, error = %e, "memory embedding failed");
                continue;
            }
        };
        let store_for_write = store.clone();
        let m = memory.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
            store_for_write
                .upsert_with_embedding(&m, &embedding)
                .map_err(|e| e.to_string())
        })
        .await;
    }
}
