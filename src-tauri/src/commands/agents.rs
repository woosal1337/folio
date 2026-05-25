//! Tauri commands for running agents against a recording's transcript.
//!
//! Each `run_agent` call:
//!   1. Loads the recording's transcript from disk
//!   2. Builds a ChatRequest with the agent's system prompt + the
//!      transcript text as the user message. If the agent is tool-using
//!      (currently only `extract-tasks`), the request also declares
//!      the `create_task` tool.
//!   3. Calls the configured LLM provider (currently always OpenAI)
//!   4. If the response contains tool calls, dispatches each one
//!      (writes to TaskStore, etc.), appends an assistant turn +
//!      Tool-role results, and re-calls the provider. Loops up to
//!      `MAX_TOOL_ITERATIONS` so a runaway model can't pin the CPU.
//!   5. Persists the final assistant text under
//!      <session_dir>/agent_runs/<agent>.json
//!   6. Returns the result to the frontend.

use std::path::{Path, PathBuf};

use attune_core::llm::agents;
use attune_core::llm::provider::LlmProvider;
use attune_core::llm::{
    Agent, AgentRun, AgentRunStore, ChatMessage, ChatRequest, ChatRole, KeyStore, OpenAiProvider,
    ProviderId, ToolCall, ToolDef,
};
use attune_core::storage::{NewTask, TaskStore};
use attune_core::transcription::SessionTranscript;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use tauri::State;
use tracing::{debug, info, warn};

use crate::app::AppState;

/// Sensible default model for the MVP. Cheap, fast, multilingual,
/// supports tool calling. Users will be able to pick per agent in a
/// later phase.
const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";

/// Soft cap on transcript characters fed to the model. ~100k chars is
/// comfortably under gpt-4o-mini's 128k token context after accounting
/// for the system prompt + the model's own output.
const TRANSCRIPT_CHAR_CAP: usize = 100_000;

/// Hard ceiling on the tool-dispatch loop. Models occasionally get
/// stuck re-calling tools with slightly different arguments; this
/// stops that from burning unbounded tokens. Five iterations is
/// generous — extract-tasks on a typical meeting needs one round of
/// 3-8 tool calls, so even a long meeting fits well inside this.
const MAX_TOOL_ITERATIONS: usize = 5;

/// Agent ids that get the `create_task` tool attached. Keeping this as
/// a small set in code rather than a field on Agent so we can wire
/// tools without forcing every custom-agent author to think about
/// schemas before the user-editable agents land.
const TASK_TOOL_AGENTS: &[&str] = &["extract-tasks"];

/// List the agents the user can invoke.
#[tauri::command]
pub fn list_agents() -> Vec<Agent> {
    debug!("list_agents");
    agents::defaults()
}

/// Load every persisted agent run for a recording. Empty vec if the
/// recording has not been processed by any agent yet.
#[tauri::command]
pub async fn list_agent_runs(session_dir: PathBuf) -> Result<Vec<AgentRun>, String> {
    let path = session_dir.clone();
    tauri::async_runtime::spawn_blocking(move || AgentRunStore::list(&path))
        .await
        .map_err(|e| format!("list_agent_runs task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

/// Delete a single saved agent run.
#[tauri::command]
pub async fn delete_agent_run(session_dir: PathBuf, agent_id: String) -> Result<(), String> {
    let path = session_dir.clone();
    let id = agent_id.clone();
    tauri::async_runtime::spawn_blocking(move || AgentRunStore::delete(&path, &id))
        .await
        .map_err(|e| format!("delete_agent_run task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

/// Run an agent against a recording.
#[tauri::command]
pub async fn run_agent(
    state: State<'_, AppState>,
    session_dir: PathBuf,
    agent_id: String,
) -> Result<AgentRun, String> {
    let agent = agents::by_id(&agent_id).ok_or_else(|| format!("unknown agent id: {agent_id}"))?;

    // Read transcript from disk on a blocking thread.
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

    let transcript_text = flatten_transcript(&transcript);
    if transcript_text.trim().is_empty() {
        return Err("transcript is empty — there is nothing for the agent to read".to_string());
    }
    let user_message = build_user_message(&transcript_text);

    // Snapshot tasks_path while holding the settings lock briefly.
    let tasks_path = state.settings.lock().tasks_path.clone();

    // Resolve provider + key.
    let provider_id = ProviderId::OpenAi;
    let api_key = tauri::async_runtime::spawn_blocking(move || KeyStore::get(provider_id))
        .await
        .map_err(|e| format!("keystore lookup panicked: {e}"))?
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            "no OpenAI API key configured. Open Settings → AI and paste your key.".to_string()
        })?;
    let provider = OpenAiProvider::new(api_key);
    let model = DEFAULT_OPENAI_MODEL.to_string();

    let tools = tools_for_agent(&agent.id);
    let session_label = session_label_from_dir(&session_dir);

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
            system_prompt: agent.system_prompt.clone(),
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

        // No tool calls → done. Capture text and break.
        if response.tool_calls.is_empty() {
            final_text = response.text;
            break;
        }

        // Tool calls present: append the assistant turn carrying the
        // calls, then dispatch each one and append its Tool-role
        // result message. Loop so the model can produce a final
        // assistant summary turn after seeing the results.
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
                session_dir.to_string_lossy().as_ref(),
                session_label.as_deref(),
            );
            if call.name == "create_task" && result.success {
                tasks_created = tasks_created.saturating_add(1);
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
            // Synthesize a closing line so the persisted run still has
            // a readable summary; better than empty text.
            final_text = format!(
                "Stopped after {} tool-call rounds. Created {} task(s) so far.",
                MAX_TOOL_ITERATIONS, tasks_created
            );
        }
    }

    // If the model's final text is empty for a tool-using agent,
    // synthesize a tiny summary so the AI page has something to render.
    if final_text.trim().is_empty() && tools.is_some() {
        final_text = if tasks_created == 0 {
            "No explicit action items found.".to_string()
        } else {
            format!(
                "Created {} task{} from this recording.",
                tasks_created,
                if tasks_created == 1 { "" } else { "s" }
            )
        };
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

    // Persist so reloading the recording brings the run back.
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
        "agent run complete",
    );
    Ok(run)
}

/// Return the tool definitions to attach to an agent's chat request, or
/// `None` if this agent doesn't use tools.
fn tools_for_agent(agent_id: &str) -> Option<Vec<ToolDef>> {
    if !TASK_TOOL_AGENTS.contains(&agent_id) {
        return None;
    }
    Some(vec![ToolDef {
        name: "create_task".to_string(),
        description: "Create a new to-do task in the user's task list. \
            Call once per distinct action item found in the meeting transcript."
            .to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Short imperative phrase describing the task (e.g. 'Send revised contract to legal')."
                },
                "owner": {
                    "type": "string",
                    "description": "Person or team responsible, exactly as named in the transcript. Omit if not stated."
                },
                "due": {
                    "type": "string",
                    "description": "Date or timeframe the task is due (e.g. 'Friday', 'next sprint', '2026-06-01'). Omit if not stated."
                },
                "notes": {
                    "type": "string",
                    "description": "Optional one-sentence context only when it materially helps a future reader."
                }
            },
            "required": ["title"],
            "additionalProperties": false
        }),
    }])
}

/// Result returned to the model in the Tool-role follow-up message.
/// Tiny + JSON-serialisable so the model can read it and (a) confirm
/// the call worked, (b) move on rather than re-trying.
#[derive(serde::Serialize)]
struct ToolResult {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Args the model passes to `create_task`. Owner/due/notes are optional
/// — we don't want a missing field to fail the call.
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

/// Dispatch a single tool call. Currently only `create_task` is
/// supported; unknown tool names return a structured error so the
/// model can recover on the next turn.
fn dispatch_tool_call(
    call: &ToolCall,
    tasks_path: &Path,
    session_dir: &str,
    session_label: Option<&str>,
) -> ToolResult {
    match call.name.as_str() {
        "create_task" => match serde_json::from_str::<CreateTaskArgs>(&call.arguments) {
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
                        error: None,
                    },
                    Err(e) => ToolResult {
                        success: false,
                        id: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            Err(e) => ToolResult {
                success: false,
                id: None,
                error: Some(format!("could not parse arguments: {e}")),
            },
        },
        other => ToolResult {
            success: false,
            id: None,
            error: Some(format!("unknown tool: {other}")),
        },
    }
}

/// Trailing path component of `session_dir`, used as the source-recording
/// label on agent-created tasks so the UI can render a back-link without
/// re-deriving it.
fn session_label_from_dir(session_dir: &Path) -> Option<String> {
    session_dir
        .file_name()
        .and_then(|os| os.to_str())
        .map(|s| s.to_string())
}

/// Concatenate every channel's segments into a single readable text
/// block. Channels (mic/system) become headings; segments are joined
/// with spaces inside each channel block.
fn flatten_transcript(transcript: &SessionTranscript) -> String {
    let mut out = String::new();
    for channel in &transcript.channels {
        if channel.segments.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        let label = match channel.channel.as_str() {
            "mic" => "[You]",
            "system" => "[Others]",
            "legacy" => "[Unknown speaker]",
            other => other,
        };
        out.push_str(label);
        out.push('\n');
        for seg in &channel.segments {
            let trimmed = seg.text.trim();
            if !trimmed.is_empty() {
                out.push_str(trimmed);
                out.push(' ');
            }
        }
    }
    out.trim().to_string()
}

/// Build the user-message payload. Truncates if absurdly long so we do
/// not OOM the provider's context window.
fn build_user_message(transcript_text: &str) -> String {
    if transcript_text.len() <= TRANSCRIPT_CHAR_CAP {
        return format!("Meeting transcript:\n\n{}", transcript_text);
    }
    let truncated = &transcript_text[..TRANSCRIPT_CHAR_CAP];
    format!(
        "Meeting transcript (truncated to first {} characters; full \
        transcript was {} characters):\n\n{}",
        TRANSCRIPT_CHAR_CAP,
        transcript_text.len(),
        truncated,
    )
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
                })
                .collect(),
        }
    }

    #[test]
    fn flatten_includes_speaker_labels() {
        let t = SessionTranscript {
            channels: vec![
                ch("mic", &["Merhaba.", "Nasılsın?"]),
                ch("system", &["İyiyim, teşekkürler."]),
            ],
        };
        let text = flatten_transcript(&t);
        assert!(text.contains("[You]"));
        assert!(text.contains("[Others]"));
        assert!(text.contains("Merhaba."));
        assert!(text.contains("İyiyim, teşekkürler."));
    }

    #[test]
    fn flatten_skips_empty_channels() {
        let t = SessionTranscript {
            channels: vec![ch("mic", &[]), ch("system", &["Single line"])],
        };
        let text = flatten_transcript(&t);
        assert!(!text.contains("[You]"));
        assert!(text.contains("[Others]"));
        assert!(text.contains("Single line"));
    }

    #[test]
    fn user_message_truncates_oversized_input() {
        let huge = "x".repeat(TRANSCRIPT_CHAR_CAP * 2);
        let msg = build_user_message(&huge);
        assert!(msg.contains("truncated to first"));
        assert!(msg.len() < TRANSCRIPT_CHAR_CAP + 500);
    }

    #[test]
    fn user_message_passes_through_normal_input() {
        let msg = build_user_message("hello world");
        assert!(msg.starts_with("Meeting transcript:"));
        assert!(msg.contains("hello world"));
        assert!(!msg.contains("truncated"));
    }

    #[test]
    fn tools_for_agent_attaches_to_extract_tasks_only() {
        assert!(tools_for_agent("extract-tasks").is_some());
        assert!(tools_for_agent("summarize").is_none());
        assert!(tools_for_agent("qa").is_none());
    }

    #[test]
    fn dispatch_create_task_writes_to_store() {
        let dir = TempDir::new().unwrap();
        let tasks_path = dir.path().join("tasks.json");
        let call = ToolCall {
            id: "call_1".into(),
            name: "create_task".into(),
            arguments: r#"{"title":"Send recap","owner":"Ege","due":"Friday"}"#.into(),
        };
        let result =
            dispatch_tool_call(&call, &tasks_path, "/sessions/abc", Some("2026-05-25-team"));
        assert!(result.success, "got error: {:?}", result.error);
        let listed = TaskStore::new(tasks_path).list();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].title, "Send recap");
        assert_eq!(listed[0].owner.as_deref(), Some("Ege"));
        assert_eq!(listed[0].due.as_deref(), Some("Friday"));
        assert!(listed[0].agent_origin);
        assert_eq!(
            listed[0].source_session_label.as_deref(),
            Some("2026-05-25-team")
        );
    }

    #[test]
    fn dispatch_create_task_rejects_bad_json() {
        let dir = TempDir::new().unwrap();
        let tasks_path = dir.path().join("tasks.json");
        let call = ToolCall {
            id: "call_x".into(),
            name: "create_task".into(),
            arguments: "{not json".into(),
        };
        let result = dispatch_tool_call(&call, &tasks_path, "/sessions/abc", None);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("could not parse"));
        assert!(TaskStore::new(tasks_path).list().is_empty());
    }

    #[test]
    fn dispatch_unknown_tool_returns_structured_error() {
        let dir = TempDir::new().unwrap();
        let tasks_path = dir.path().join("tasks.json");
        let call = ToolCall {
            id: "call_x".into(),
            name: "delete_universe".into(),
            arguments: "{}".into(),
        };
        let result = dispatch_tool_call(&call, &tasks_path, "/sessions/abc", None);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("unknown tool"));
    }
}
