//! OpenAI Chat Completions backend.
//!
//! Hits `/v1/chat/completions`. Phase 1 is non-streaming only; phase 5
//! adds the SSE streaming path on top of the same provider.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::{AttuneError, Result};
use crate::llm::provider::{LlmProvider, ProviderId};
use crate::llm::types::{ChatRequest, ChatResponse, ChatRole, FinishReason, ModelInfo, ToolCall};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

/// OpenAI Chat Completions backend.
pub struct OpenAiProvider {
    api_key: String,
    base_url: String,
    client: Client,
}

impl OpenAiProvider {
    /// New provider with the default base URL (`api.openai.com`).
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_base_url(api_key, DEFAULT_BASE_URL)
    }

    /// New provider with a custom base URL. Used by the DeepSeek
    /// provider in phase 2 (DeepSeek is OpenAI-compatible) and by
    /// tests that point at a recorded fixture server.
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: Client::new(),
        }
    }

    /// Privacy-mode gate (GET-174): refuse any outbound request when the
    /// host is blocked (airgap / Privacy Mode; localhost stays allowed).
    /// Called before every `send()` so chat transcript content, model
    /// listing, and key tests all honour the cutoff — mirrors the
    /// transcription + embedding gating.
    fn ensure_egress_allowed(&self) -> Result<()> {
        let host = crate::cloud_guard::host_of(&self.base_url).unwrap_or_default();
        crate::cloud_guard::ensure_allowed(host).map_err(|e| AttuneError::Llm(e.to_string()))
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn id(&self) -> ProviderId {
        ProviderId::OpenAi
    }

    async fn test(&self) -> Result<()> {
        self.ensure_egress_allowed()?;
        let url = format!("{}/models", self.base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| AttuneError::Llm(format!("openai /models request failed: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AttuneError::Llm(format!(
                "openai /models returned HTTP {status}: {body}"
            )));
        }
        Ok(())
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        self.ensure_egress_allowed()?;
        let url = format!("{}/models", self.base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| AttuneError::Llm(format!("openai /models request failed: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AttuneError::Llm(format!(
                "openai /models returned HTTP {status}: {body}"
            )));
        }
        let parsed: ModelsResponse = resp
            .json()
            .await
            .map_err(|e| AttuneError::Llm(format!("openai /models json decode failed: {e}")))?;

        // Filter to the chat-capable families. The /models endpoint
        // returns every model the org has access to, including
        // embeddings/tts/dalle entries that the UI does not want to
        // show in the chat-model picker.
        let mut models: Vec<ModelInfo> = parsed
            .data
            .into_iter()
            .filter(|m| is_chat_model(&m.id))
            .map(|m| ModelInfo {
                id: m.id.clone(),
                display_name: m.id,
                context_window: 0, // OpenAI does not return this; future phase fills it from a static table
            })
            .collect();

        // Stable order so the UI dropdown does not jump around between
        // refreshes.
        models.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(models)
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        self.ensure_egress_allowed()?;
        let body = build_chat_request_body(&request);
        debug!(
            model = %request.model,
            messages = request.messages.len(),
            "openai chat request",
        );
        let url = format!("{}/chat/completions", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                AttuneError::Llm(format!("openai /chat/completions request failed: {e}"))
            })?;
        let status = resp.status();
        if !status.is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            return Err(AttuneError::Llm(format!(
                "openai /chat/completions returned HTTP {status}: {err_body}"
            )));
        }
        let parsed: ChatCompletionResponse = resp.json().await.map_err(|e| {
            AttuneError::Llm(format!("openai /chat/completions json decode failed: {e}"))
        })?;
        let choice = parsed.choices.into_iter().next().ok_or_else(|| {
            AttuneError::Llm("openai /chat/completions returned zero choices".to_string())
        })?;
        let tool_calls: Vec<ToolCall> = choice
            .message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|c| ToolCall {
                id: c.id,
                name: c.function.name,
                arguments: c.function.arguments,
            })
            .collect();
        Ok(ChatResponse {
            text: choice.message.content.unwrap_or_default(),
            finish_reason: parse_finish_reason(choice.finish_reason.as_deref()),
            prompt_tokens: parsed.usage.as_ref().map(|u| u.prompt_tokens),
            completion_tokens: parsed.usage.as_ref().map(|u| u.completion_tokens),
            tool_calls,
        })
    }
}

fn build_chat_request_body(req: &ChatRequest) -> ChatCompletionRequestBody {
    let mut messages: Vec<OpenAiMessage> = Vec::with_capacity(req.messages.len() + 1);
    if !req.system_prompt.is_empty() {
        messages.push(OpenAiMessage {
            role: "system".to_string(),
            content: Some(req.system_prompt.clone()),
            tool_calls: None,
            tool_call_id: None,
        });
    }
    for m in &req.messages {
        messages.push(OpenAiMessage {
            role: role_to_str(m.role).to_string(),
            content: if m.content.is_empty() && m.tool_calls.is_some() {
                // Assistant turn that was purely tool calls: OpenAI's
                // schema permits content == null in that case.
                None
            } else {
                Some(m.content.clone())
            },
            tool_calls: m.tool_calls.as_ref().map(|calls| {
                calls
                    .iter()
                    .map(|c| OpenAiToolCall {
                        id: c.id.clone(),
                        kind: "function".to_string(),
                        function: OpenAiToolCallFunction {
                            name: c.name.clone(),
                            arguments: c.arguments.clone(),
                        },
                    })
                    .collect()
            }),
            tool_call_id: m.tool_call_id.clone(),
        });
    }
    let tools = req.tools.as_ref().map(|defs| {
        defs.iter()
            .map(|t| OpenAiToolDef {
                kind: "function".to_string(),
                function: OpenAiToolDefFunction {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: t.parameters.clone(),
                },
            })
            .collect()
    });
    ChatCompletionRequestBody {
        model: req.model.clone(),
        messages,
        temperature: req.temperature,
        max_tokens: req.max_tokens,
        tools,
    }
}

fn role_to_str(role: ChatRole) -> &'static str {
    match role {
        ChatRole::System => "system",
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::Tool => "tool",
    }
}

fn parse_finish_reason(s: Option<&str>) -> FinishReason {
    match s {
        Some("stop") => FinishReason::Stop,
        Some("length") => FinishReason::Length,
        Some("tool_calls") => FinishReason::ToolCall,
        _ => FinishReason::Stop,
    }
}

fn is_chat_model(id: &str) -> bool {
    // Conservative allow-by-prefix list. We do NOT use a deny list
    // because OpenAI adds new model families all the time and we
    // want them to surface in the picker automatically.
    const CHAT_PREFIXES: &[&str] = &["gpt-", "o1", "o3", "o4", "chatgpt-", "openai/gpt-"];
    CHAT_PREFIXES.iter().any(|p| id.starts_with(p))
}

#[derive(Serialize)]
struct ChatCompletionRequestBody {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiToolDef>>,
}

#[derive(Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    function: OpenAiToolCallFunction,
}

#[derive(Serialize, Deserialize)]
struct OpenAiToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct OpenAiToolDef {
    #[serde(rename = "type")]
    kind: String,
    function: OpenAiToolDefFunction,
}

#[derive(Serialize)]
struct OpenAiToolDefFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Choice {
    message: OpenAiMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::{ChatMessage, ChatRequest, ChatRole, ToolDef};
    use serde_json::json;

    #[test]
    fn egress_host_resolves_to_openai_for_the_airgap_guard() {
        // GET-174: every send() gates on this host via cloud_guard, so it
        // must be the real OpenAI host for Privacy Mode to block it. Paired
        // with cloud_guard::tests::airgap_blocks_external_hosts; avoids
        // mutating the process-global airgap flag (which races parallel
        // tests), mirroring the embed.rs approach.
        let provider = OpenAiProvider::new("sk-test");
        let host = crate::cloud_guard::host_of(&provider.base_url).expect("host must parse");
        assert_eq!(host, "api.openai.com");
    }

    fn user(text: &str) -> ChatMessage {
        ChatMessage {
            role: ChatRole::User,
            content: text.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    #[test]
    fn build_request_body_prepends_system_prompt() {
        let req = ChatRequest {
            model: "gpt-5".to_string(),
            system_prompt: "you are a test".to_string(),
            messages: vec![user("hi")],
            temperature: Some(0.2),
            max_tokens: Some(64),
            tools: None,
        };
        let body = build_chat_request_body(&req);
        assert_eq!(body.model, "gpt-5");
        assert_eq!(body.messages.len(), 2);
        assert_eq!(body.messages[0].role, "system");
        assert_eq!(body.messages[0].content.as_deref(), Some("you are a test"));
        assert_eq!(body.messages[1].role, "user");
        assert_eq!(body.messages[1].content.as_deref(), Some("hi"));
        assert_eq!(body.temperature, Some(0.2));
        assert_eq!(body.max_tokens, Some(64));
        assert!(body.tools.is_none());
    }

    #[test]
    fn build_request_body_skips_empty_system_prompt() {
        let req = ChatRequest {
            model: "gpt-5".to_string(),
            system_prompt: "".to_string(),
            messages: vec![user("hi")],
            temperature: None,
            max_tokens: None,
            tools: None,
        };
        let body = build_chat_request_body(&req);
        assert_eq!(body.messages.len(), 1);
        assert_eq!(body.messages[0].role, "user");
    }

    #[test]
    fn build_request_body_serialises_tools() {
        let req = ChatRequest {
            model: "gpt-5".to_string(),
            system_prompt: "".to_string(),
            messages: vec![user("extract action items")],
            temperature: None,
            max_tokens: None,
            tools: Some(vec![ToolDef {
                name: "create_task".to_string(),
                description: "Create a new to-do item.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                    },
                    "required": ["title"],
                }),
            }]),
        };
        let body = build_chat_request_body(&req);
        let tools = body.tools.expect("tools should serialise");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].kind, "function");
        assert_eq!(tools[0].function.name, "create_task");
    }

    #[test]
    fn chat_model_filter_keeps_gpt_and_o_series() {
        assert!(is_chat_model("gpt-5"));
        assert!(is_chat_model("gpt-4o"));
        assert!(is_chat_model("o1-mini"));
        assert!(is_chat_model("o3"));
        assert!(is_chat_model("chatgpt-4o-latest"));
    }

    #[test]
    fn chat_model_filter_drops_non_chat() {
        assert!(!is_chat_model("text-embedding-3-large"));
        assert!(!is_chat_model("whisper-1"));
        assert!(!is_chat_model("dall-e-3"));
        assert!(!is_chat_model("tts-1"));
    }

    #[test]
    fn finish_reason_normalises_known_values() {
        assert_eq!(parse_finish_reason(Some("stop")), FinishReason::Stop);
        assert_eq!(parse_finish_reason(Some("length")), FinishReason::Length);
        assert_eq!(
            parse_finish_reason(Some("tool_calls")),
            FinishReason::ToolCall
        );
        assert_eq!(parse_finish_reason(None), FinishReason::Stop);
        assert_eq!(parse_finish_reason(Some("???")), FinishReason::Stop);
    }
}
