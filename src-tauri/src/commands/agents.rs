//! Tauri commands for running agents against a recording's transcript.
//!
//! Phase 1.5 MVP: one-shot agent runs only (no multi-turn chat). Each
//! command call:
//!   1. Loads the recording's transcript from disk
//!   2. Builds a ChatRequest with the agent's system prompt + the
//!      transcript text as the user message
//!   3. Calls the configured LLM provider (currently always OpenAI)
//!   4. Persists the result under <session_dir>/agent_runs/<agent>.json
//!   5. Returns the result to the frontend
//!
//! Multi-turn chat, streaming, tool calling, provider switching, and
//! custom agent editing land in phases 3-9 per the vault plan.

use std::path::PathBuf;

use attune_core::llm::agents;
use attune_core::llm::provider::LlmProvider;
use attune_core::llm::{
    Agent, AgentRun, AgentRunStore, ChatMessage, ChatRequest, ChatRole, KeyStore, OpenAiProvider,
    ProviderId,
};
use attune_core::transcription::SessionTranscript;
use chrono::Utc;
use tracing::{debug, info};

/// Sensible default model for the MVP. Cheap, fast, multilingual.
/// Users will be able to pick per agent in phase 6.
const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";

/// Soft cap on transcript characters fed to the model. ~100k chars is
/// comfortably under gpt-4o-mini's 128k token context after accounting
/// for the system prompt and the model's own output. Long transcripts
/// get truncated with a notice prepended; phase 3 will replace this
/// with proper token counting + downsampling.
const TRANSCRIPT_CHAR_CAP: usize = 100_000;

/// List the agents the user can invoke. Phase 1.5 returns only the
/// four baked-in defaults. Phase 3 will read TOML files from the vault
/// and merge them in.
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

/// Run an agent against a recording. Loads the transcript, calls the
/// provider, persists, returns. Synchronous from the frontend's
/// perspective — phase 5 adds streaming.
#[tauri::command]
pub async fn run_agent(session_dir: PathBuf, agent_id: String) -> Result<AgentRun, String> {
    // Look up the agent.
    let agent = agents::by_id(&agent_id).ok_or_else(|| format!("unknown agent id: {agent_id}"))?;

    // Read transcript from disk on a blocking thread (file I/O).
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

    // Resolve provider + key. MVP hardcodes OpenAI; phase 6 settings
    // will let the user pick.
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
    let request = ChatRequest {
        model: model.clone(),
        system_prompt: agent.system_prompt.clone(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: user_message,
        }],
        temperature: Some(0.2),
        max_tokens: None,
    };

    info!(
        agent = %agent.id,
        provider = provider_id.as_str(),
        model = %model,
        transcript_chars = transcript_text.len(),
        "running agent",
    );
    let response = provider.chat(request).await.map_err(|e| e.to_string())?;

    let run = AgentRun {
        agent_id: agent.id.clone(),
        agent_name: agent.name.clone(),
        provider: provider_id,
        model,
        response: response.text,
        prompt_tokens: response.prompt_tokens,
        completion_tokens: response.completion_tokens,
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
        "agent run complete",
    );
    Ok(run)
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
}
