//! Provider-agnostic types for LLM chat.
//!
//! These cross the Tauri IPC boundary so they derive [`TS`] and the
//! generated TypeScript lands under `src/shared/types/`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::llm::ProviderId;

/// One message in a chat conversation.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

/// Conversation role. We intentionally do not model tool messages in
/// phase 1 — they arrive with tool dispatch in phase 9.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/shared/types/")]
pub enum ChatRole {
    System,
    User,
    Assistant,
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
}

/// A non-streaming chat response.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ChatResponse {
    pub text: String,
    pub finish_reason: FinishReason,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
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
