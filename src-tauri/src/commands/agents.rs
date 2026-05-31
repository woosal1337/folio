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

use std::path::{Path, PathBuf};

use attune_core::llm::agents;
use attune_core::llm::provider::LlmProvider;
use attune_core::llm::{
    Agent, AgentRun, AgentRunStore, ChatMessage, ChatRequest, ChatRole, KeyStore, OpenAiProvider,
    ProviderId, ToolCall, ToolDef,
};
use attune_core::memory::{EmbeddingClient, MemoryKind, MemoryStore, NewMemory};
use attune_core::storage::{NewTask, TaskStore};
use attune_core::transcription::SessionTranscript;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use tauri::State;
use tracing::{debug, info, warn};

use crate::app::AppState;

const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";
const TRANSCRIPT_CHAR_CAP: usize = 100_000;
const MAX_TOOL_ITERATIONS: usize = 5;

/// Build the language trailer appended to every agent's system
/// prompt. Closes v2 roadmap finding R09 / implements 097.
///
/// `briefing_language` is the user's Settings choice:
///   * `"auto"` → mirror the meeting's language (legacy behaviour).
///   * any other BCP-47 tag → force that language regardless of the
///     transcript, including tool-call free-text fields (task titles,
///     memory content, autoname title/subtitle).
///
/// We do not auto-detect from the transcript ourselves — the model
/// picks up the meeting's dominant script from the user message that
/// follows. The trailer just states the rule.
fn language_aware_trailer(briefing_language: &str) -> String {
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

/// Map a BCP-47 / ISO-639-1 tag to the English name the model knows
/// best. Unknown tags pass through verbatim so a niche tag like `cy`
/// (Welsh) still produces a sensible instruction ("Always reply in cy")
/// rather than crashing — the model will treat the tag as a language
/// hint regardless.
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

/// Agents that receive the `create_task` tool.
const TASK_TOOL_AGENTS: &[&str] = &["extract-tasks"];

/// Agents that receive the `remember` tool.
const MEMORY_WRITE_TOOL_AGENTS: &[&str] = &["extract-memories"];

/// Agents that receive the `search_memory` tool. We give it to every
/// agent: a summary or Q&A turn benefits from pulling prior context
/// ("user said last week they prefer async standups"), and the
/// extract-* agents benefit from deduping against what's already
/// remembered before writing.
fn memory_search_for_all() -> bool {
    true
}

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

    let transcript_text = flatten_transcript(&session_dir, &transcript);
    if transcript_text.trim().is_empty() {
        return Err("transcript is empty — there is nothing for the agent to read".to_string());
    }
    // GET-147: the summary folds in the notes the user typed live during
    // the call so action items / decisions they captured seed the
    // structured note instead of being lost. Other agents read the
    // transcript only.
    let live_notes_md = if matches!(agent.id.as_str(), "summarize" | "write-followup-email") {
        let dir = session_dir.clone();
        tauri::async_runtime::spawn_blocking(move || read_live_notes_markdown(&dir))
            .await
            .unwrap_or(None)
    } else {
        None
    };
    let user_message = build_user_message(&transcript_text, live_notes_md.as_deref());

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

    let tools = tools_for_agent(&agent.id);
    let session_label = session_label_from_dir(&session_dir);

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
    let system_prompt = format!("{base}{}", language_aware_trailer(&briefing_language));

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
            let result = dispatch_tool_call(
                call,
                &tasks_path,
                memory_store.clone(),
                session_dir.to_string_lossy().as_ref(),
                session_label.as_deref(),
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
        final_text = synth_summary(&agent.id, tasks_created, memories_created.len());
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

/// Build the tool definitions an agent gets. Order is stable so the
/// model sees `search_memory` first (it's the read tool) and writes
/// after.
fn tools_for_agent(agent_id: &str) -> Option<Vec<ToolDef>> {
    let mut tools = Vec::new();
    if memory_search_for_all() {
        tools.push(search_memory_tool_def());
    }
    if TASK_TOOL_AGENTS.contains(&agent_id) {
        tools.push(create_task_tool_def());
    }
    if MEMORY_WRITE_TOOL_AGENTS.contains(&agent_id) {
        tools.push(remember_tool_def());
    }
    if tools.is_empty() {
        None
    } else {
        Some(tools)
    }
}

fn create_task_tool_def() -> ToolDef {
    ToolDef {
        name: "create_task".to_string(),
        description: "Create a new to-do task in the user's task list. \
            Call once per distinct action item found in the meeting transcript."
            .to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "Short imperative phrase describing the task." },
                "owner": { "type": "string", "description": "Person or team responsible. Omit if not stated." },
                "due":   { "type": "string", "description": "Date or timeframe. Omit if not stated." },
                "notes": { "type": "string", "description": "Optional one-sentence context." }
            },
            "required": ["title"],
            "additionalProperties": false
        }),
    }
}

fn remember_tool_def() -> ToolDef {
    ToolDef {
        name: "remember".to_string(),
        description: "Capture a lasting fact about the user, their projects, or the people they work with. \
Call once per fact. Use `claim` for facts about the user, `pref` for preferences, `person` for someone they collaborate with, \
`observe` for free-form context with no obvious key. Conflicting facts on the same key supersede automatically; do not try to deduplicate."
            .to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "kind": {
                    "type": "string",
                    "enum": ["claim", "pref", "person", "observe"],
                    "description": "claim / pref / person / observe — see the agent prompt for guidance."
                },
                "key": {
                    "type": "string",
                    "description": "Dotted handle (e.g. `user.company`, `ui.theme`, `person.alice`). Required for claim/pref/person; omit for observe."
                },
                "content": {
                    "type": "string",
                    "description": "The fact in one sentence, present tense."
                },
                "evidence": {
                    "type": "string",
                    "description": "Short quoted snippet from the transcript that supports the fact."
                },
                "confidence": {
                    "type": "number",
                    "description": "0.0-1.0; under 0.6 means \"plausible but unsure\"."
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "1-4 short lowercase tags."
                }
            },
            "required": ["kind", "content"],
            "additionalProperties": false
        }),
    }
}

fn search_memory_tool_def() -> ToolDef {
    ToolDef {
        name: "search_memory".to_string(),
        description: "Look up what the system already knows about the user. \
CALL THIS WHENEVER: you need to verify a name/role/company, check whether a topic has come up before, \
or avoid re-asking something the user has stated previously. Returns up to `limit` currently-valid memories ranked by relevance."
            .to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Free-text search. Use the key (e.g. `user.company`) or the topic (e.g. `quarterly planning`)."
                },
                "kinds": {
                    "type": "array",
                    "items": { "type": "string", "enum": ["claim", "pref", "person", "observe"] },
                    "description": "Optional. Restrict to these kinds. Omit to search all."
                },
                "limit": {
                    "type": "integer",
                    "description": "Optional. Max rows to return (default 5)."
                }
            },
            "required": ["query"],
            "additionalProperties": false
        }),
    }
}

#[derive(serde::Serialize, Default)]
struct ToolResult {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct CreateTaskArgs {
    title: String,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    due: Option<String>,
    #[serde(default)]
    notes: Option<String>,
}

#[derive(Deserialize)]
struct RememberArgs {
    kind: String,
    #[serde(default)]
    key: Option<String>,
    content: String,
    #[serde(default)]
    evidence: Option<String>,
    #[serde(default = "default_remember_confidence")]
    confidence: f32,
    #[serde(default)]
    tags: Vec<String>,
}

fn default_remember_confidence() -> f32 {
    0.8
}

#[derive(Deserialize)]
struct SearchMemoryArgs {
    query: String,
    #[serde(default)]
    kinds: Vec<String>,
    #[serde(default)]
    limit: Option<usize>,
}

fn dispatch_tool_call(
    call: &ToolCall,
    tasks_path: &Path,
    memory_store: std::sync::Arc<MemoryStore>,
    session_dir: &str,
    session_label: Option<&str>,
) -> ToolResult {
    match call.name.as_str() {
        "create_task" => dispatch_create_task(call, tasks_path, session_dir, session_label),
        "remember" => dispatch_remember(call, memory_store, session_dir, session_label),
        "search_memory" => dispatch_search_memory(call, memory_store),
        other => ToolResult {
            success: false,
            error: Some(format!("unknown tool: {other}")),
            ..ToolResult::default()
        },
    }
}

fn dispatch_create_task(
    call: &ToolCall,
    tasks_path: &Path,
    session_dir: &str,
    session_label: Option<&str>,
) -> ToolResult {
    match serde_json::from_str::<CreateTaskArgs>(&call.arguments) {
        Ok(args) => {
            let store = TaskStore::new(tasks_path.to_path_buf());
            let new_task = NewTask {
                title: args.title,
                status: None,
                owner: args.owner.filter(|s| !s.trim().is_empty()),
                due: args.due.filter(|s| !s.trim().is_empty()),
                notes: args.notes.filter(|s| !s.trim().is_empty()),
                source_session_dir: Some(session_dir.to_string()),
                source_session_label: session_label.map(|s| s.to_string()),
                agent_origin: true,
            };
            match store.create(new_task) {
                Ok(task) => ToolResult {
                    success: true,
                    id: Some(task.id),
                    ..ToolResult::default()
                },
                Err(e) => ToolResult {
                    success: false,
                    error: Some(e.to_string()),
                    ..ToolResult::default()
                },
            }
        }
        Err(e) => ToolResult {
            success: false,
            error: Some(format!("could not parse arguments: {e}")),
            ..ToolResult::default()
        },
    }
}

fn dispatch_remember(
    call: &ToolCall,
    store: std::sync::Arc<MemoryStore>,
    session_dir: &str,
    session_label: Option<&str>,
) -> ToolResult {
    let args = match serde_json::from_str::<RememberArgs>(&call.arguments) {
        Ok(a) => a,
        Err(e) => {
            return ToolResult {
                success: false,
                error: Some(format!("could not parse arguments: {e}")),
                ..ToolResult::default()
            }
        }
    };
    let kind = match MemoryKind::parse(&args.kind) {
        Some(k) => k,
        None => {
            return ToolResult {
                success: false,
                error: Some(format!("unknown kind: {}", args.kind)),
                ..ToolResult::default()
            }
        }
    };
    let new_memory = NewMemory {
        kind,
        key: args.key.filter(|s| !s.trim().is_empty()),
        content: args.content,
        evidence: args.evidence.filter(|s| !s.trim().is_empty()),
        confidence: args.confidence,
        tags: args.tags,
        source_session_dir: Some(session_dir.to_string()),
        source_session_label: session_label.map(|s| s.to_string()),
    };
    match store.create(new_memory) {
        Ok(outcome) => {
            let memory = outcome.into_memory();
            ToolResult {
                success: true,
                id: Some(memory.id),
                ..ToolResult::default()
            }
        }
        Err(e) => ToolResult {
            success: false,
            error: Some(e.to_string()),
            ..ToolResult::default()
        },
    }
}

fn dispatch_search_memory(call: &ToolCall, store: std::sync::Arc<MemoryStore>) -> ToolResult {
    let args = match serde_json::from_str::<SearchMemoryArgs>(&call.arguments) {
        Ok(a) => a,
        Err(e) => {
            return ToolResult {
                success: false,
                error: Some(format!("could not parse arguments: {e}")),
                ..ToolResult::default()
            }
        }
    };
    let kinds: Vec<MemoryKind> = args
        .kinds
        .iter()
        .filter_map(|s| MemoryKind::parse(s))
        .collect();
    let limit = args.limit.unwrap_or(5);
    // Embedding-free path inside the dispatcher to keep tool calls
    // synchronous; FTS5 BM25 alone is enough signal for the kinds of
    // lookups agents do mid-reasoning.
    match store.search(&args.query, None, &kinds, limit) {
        Ok(memories) => {
            // Project to a tiny shape so the model isn't drowning in
            // timestamps and ids.
            let projected: Vec<serde_json::Value> = memories
                .into_iter()
                .map(|m| {
                    json!({
                        "id": m.id,
                        "kind": m.kind.as_str(),
                        "key": m.key,
                        "content": m.content,
                        "valid_from": m.valid_from.to_rfc3339(),
                    })
                })
                .collect();
            ToolResult {
                success: true,
                data: Some(json!({ "results": projected })),
                ..ToolResult::default()
            }
        }
        Err(e) => ToolResult {
            success: false,
            error: Some(e.to_string()),
            ..ToolResult::default()
        },
    }
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

fn synth_summary(agent_id: &str, tasks: usize, memories: usize) -> String {
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

fn session_label_from_dir(session_dir: &Path) -> Option<String> {
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
fn flatten_transcript(session_dir: &Path, transcript: &SessionTranscript) -> String {
    let names = attune_core::diarization::SessionSpeakers::read(session_dir)
        .ok()
        .flatten()
        .map(|s| s.name_map())
        .unwrap_or_default();
    transcript.to_labeled_dialogue_named(false, &names)
}

fn build_user_message(transcript_text: &str, live_notes_md: Option<&str>) -> String {
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
        let truncated = &transcript_text[..TRANSCRIPT_CHAR_CAP];
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
    out
}

/// Read a session's live notes (GET-145) and render them as the grouped
/// markdown the summary agent folds in. None when the session has no
/// notes or the file is missing/unreadable.
fn read_live_notes_markdown(session_dir: &Path) -> Option<String> {
    let bytes = std::fs::read(session_dir.join("live_notes.json")).ok()?;
    let lines: Vec<attune_core::live_notes::RawNoteLine> = serde_json::from_slice(&bytes).ok()?;
    let notes = attune_core::live_notes::parse_lines(&lines);
    if notes.is_empty() {
        return None;
    }
    Some(attune_core::live_notes::render_markdown(&notes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use attune_core::transcription::{ChannelTranscript, SessionTranscript, TranscriptSegment};
    use tempfile::TempDir;

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
        // Niche language tags still produce a usable instruction —
        // worst case the model treats the tag as a language hint.
        let t = language_aware_trailer("cy");
        assert!(t.contains("Always reply in cy"));
    }

    #[test]
    fn trailer_evidence_snippet_rule_only_when_forcing_translation() {
        // The evidence-snippet carve-out is only meaningful when the
        // briefing language differs from the transcript language;
        // auto mode has no such concept.
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
        // The mic channel is the note-taker ("You:"); un-diarized system
        // audio falls back to "Others:".
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
        let msg = build_user_message(&huge, None);
        assert!(msg.contains("truncated to first"));
        assert!(msg.len() < TRANSCRIPT_CHAR_CAP + 500);
    }

    #[test]
    fn user_message_appends_live_notes_block_when_present() {
        let msg = build_user_message("hello", Some("## Action items\n\n- `0:05` ship"));
        assert!(msg.contains("<user_live_notes>"));
        assert!(msg.contains("## Action items"));
        assert!(msg.contains("ship"));
        // Empty / whitespace notes add no block.
        let bare = build_user_message("hello", Some("   "));
        assert!(!bare.contains("<user_live_notes>"));
    }

    #[test]
    fn tools_for_agent_attaches_correct_set() {
        let summarize = tools_for_agent("summarize").unwrap();
        assert!(summarize.iter().any(|t| t.name == "search_memory"));
        assert!(!summarize.iter().any(|t| t.name == "create_task"));
        assert!(!summarize.iter().any(|t| t.name == "remember"));

        let tasks = tools_for_agent("extract-tasks").unwrap();
        assert!(tasks.iter().any(|t| t.name == "create_task"));
        assert!(tasks.iter().any(|t| t.name == "search_memory"));

        let memories = tools_for_agent("extract-memories").unwrap();
        assert!(memories.iter().any(|t| t.name == "remember"));
        assert!(memories.iter().any(|t| t.name == "search_memory"));
    }

    /// Helper: open a MemoryStore at a tempdir and return the
    /// `Arc<MemoryStore>` the dispatch helpers now expect.
    fn arc_store(dir: &std::path::Path) -> std::sync::Arc<MemoryStore> {
        std::sync::Arc::new(MemoryStore::open(dir).unwrap())
    }

    #[test]
    fn dispatch_create_task_writes_to_store() {
        let dir = TempDir::new().unwrap();
        let tasks_path = dir.path().join("tasks.json");
        let memory_dir = dir.path().join("memory");
        let store = arc_store(&memory_dir);
        let call = ToolCall {
            id: "call_1".into(),
            name: "create_task".into(),
            arguments: r#"{"title":"Send recap","owner":"Ege"}"#.into(),
        };
        let r = dispatch_tool_call(
            &call,
            &tasks_path,
            store,
            "/sessions/abc",
            Some("2026-05-25-team"),
        );
        assert!(r.success, "got error: {:?}", r.error);
        assert!(r.id.is_some());
    }

    #[test]
    fn dispatch_remember_creates_memory() {
        let dir = TempDir::new().unwrap();
        let tasks_path = dir.path().join("tasks.json");
        let memory_dir = dir.path().join("memory");
        let store = arc_store(&memory_dir);
        let call = ToolCall {
            id: "call_m".into(),
            name: "remember".into(),
            arguments: r#"{"kind":"claim","key":"user.company","content":"Attune","confidence":0.9,"tags":["company"]}"#.into(),
        };
        let r = dispatch_tool_call(
            &call,
            &tasks_path,
            store.clone(),
            "/sessions/abc",
            Some("2026-05-25"),
        );
        assert!(r.success, "got error: {:?}", r.error);
        let memories = store.list(&Default::default()).unwrap();
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].content, "Attune");
        assert!(memories[0].source_session_dir.is_some());
    }

    #[test]
    fn dispatch_search_memory_returns_hits() {
        let dir = TempDir::new().unwrap();
        let tasks_path = dir.path().join("tasks.json");
        let memory_dir = dir.path().join("memory");
        let store = arc_store(&memory_dir);
        // Seed one memory.
        store
            .create(NewMemory {
                kind: MemoryKind::Claim,
                key: Some("user.company".into()),
                content: "Attune".into(),
                ..NewMemory::default()
            })
            .unwrap();
        let call = ToolCall {
            id: "call_s".into(),
            name: "search_memory".into(),
            arguments: r#"{"query":"company","limit":5}"#.into(),
        };
        let r = dispatch_tool_call(&call, &tasks_path, store, "/sessions/abc", None);
        assert!(r.success, "got error: {:?}", r.error);
        let data = r.data.expect("has data");
        let results = data.get("results").and_then(|v| v.as_array()).unwrap();
        assert_eq!(results.len(), 1);
    }
}
