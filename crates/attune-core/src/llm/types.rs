//! Provider-agnostic types for LLM chat.
//!
//! These cross the Tauri IPC boundary so they derive [`TS`] and the
//! generated TypeScript lands under `src/shared/types/`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::llm::ProviderId;

/// One message in a chat conversation. `content` may be empty when the
/// assistant turn is purely a tool call. `tool_calls` carries the
/// model's request to invoke one or more declared tools. `tool_call_id`
/// pairs a `Tool`-role message back to the call it answers.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Conversation role. `Tool` carries the result of a tool invocation
/// back to the model in a follow-up turn.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/shared/types/")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A function/tool the model is allowed to invoke. The schema is a
/// JSON Schema fragment (passed straight through to the provider) so
/// the model knows the argument shape.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    /// JSON Schema describing the tool's arguments object. Stored as
    /// a `serde_json::Value` so callers can construct it with the
    /// `serde_json::json!` macro and we don't have to model every
    /// JSON Schema construct in Rust.
    pub parameters: serde_json::Value,
}

/// A single tool invocation requested by the model.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ToolCall {
    /// Provider-assigned id (e.g. OpenAI's `call_abc123`). Used to
    /// pair the `Tool`-role response message back to the call.
    pub id: String,
    /// Name of the tool the model wants to invoke. Must match a
    /// `ToolDef.name` from the request.
    pub name: String,
    /// JSON-encoded arguments string exactly as the model produced
    /// it. We do not pre-parse here so the dispatcher can validate
    /// and surface schema errors back to the model on the next turn.
    pub arguments: String,
}

/// Why a generation stopped. Mostly a courtesy for the UI; we always
/// surface the partial response either way.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../src/shared/types/")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCall,
    Error,
}

/// Inputs for a single chat completion.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub system_prompt: String,
    pub messages: Vec<ChatMessage>,
    /// 0.0..=1.0. None falls back to the provider's default.
    pub temperature: Option<f32>,
    /// Cap on output tokens. None falls back to the provider's default.
    pub max_tokens: Option<u32>,
    /// Tools the model is allowed to invoke. Empty / None means a
    /// plain chat completion. The agent runner is responsible for
    /// dispatching any tool calls in the response and looping until
    /// the model finishes with a plain assistant message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDef>>,
}

/// A non-streaming chat response. When `tool_calls` is non-empty the
/// model is asking the caller to invoke one or more tools and feed the
/// results back as `Tool`-role messages on the next turn.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ChatResponse {
    pub text: String,
    pub finish_reason: FinishReason,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
}

/// Static metadata about a provider's model.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub context_window: u32,
}

/// Non-secret configuration for a provider. The API key lives in the
/// keychain, not here.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ProviderConfig {
    pub provider: ProviderId,
    pub base_url: String,
    pub default_model: String,
}

/// Status of one provider for the Settings UI. Returned by
/// `list_providers` Tauri command.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ProviderStatus {
    pub id: ProviderId,
    pub display_name: String,
    /// True if a key is present in the keychain.
    pub configured: bool,
    /// Last 4 characters of the stored key (e.g. "…AB12"), if any.
    pub redacted_suffix: Option<String>,
    /// True if this provider is the one we recommend in onboarding.
    /// Currently OpenAI per the vault plan.
    pub recommended: bool,
}
