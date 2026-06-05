use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::llm::ProviderId;

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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/shared/types/")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,

    pub parameters: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ToolCall {
    pub id: String,

    pub name: String,

    pub arguments: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../src/shared/types/")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCall,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub system_prompt: String,
    pub messages: Vec<ChatMessage>,

    pub temperature: Option<f32>,

    pub max_tokens: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDef>>,
}

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

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub context_window: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ProviderConfig {
    pub provider: ProviderId,
    pub base_url: String,
    pub default_model: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ProviderStatus {
    pub id: ProviderId,
    pub display_name: String,

    pub configured: bool,

    pub redacted_suffix: Option<String>,

    pub recommended: bool,
}
