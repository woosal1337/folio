//! The [`LlmProvider`] trait every chat backend implements.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::Result;
use crate::llm::types::{ChatRequest, ChatResponse, ModelInfo};

/// Stable identifier for each provider we ship support for.
///
/// Wire-encoded as lowercase ("openai", "anthropic", "deepseek") so
/// JSON files and keychain entries stay stable across renames in the
/// Rust source.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/shared/types/")]
pub enum ProviderId {
    OpenAi,
    Anthropic,
    Deepseek,
}

impl ProviderId {
    /// All providers known to the build, in onboarding-recommended
    /// order. OpenAI is the recommended default per the vault plan.
    pub fn all() -> &'static [ProviderId] {
        &[
            ProviderId::OpenAi,
            ProviderId::Anthropic,
            ProviderId::Deepseek,
        ]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ProviderId::OpenAi => "openai",
            ProviderId::Anthropic => "anthropic",
            ProviderId::Deepseek => "deepseek",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            ProviderId::OpenAi => "OpenAI",
            ProviderId::Anthropic => "Anthropic",
            ProviderId::Deepseek => "DeepSeek",
        }
    }
}

/// One backend that can run a chat completion.
///
/// Implementations live under [`crate::llm::providers`]. Each is
/// responsible for normalising provider-specific request/response
/// shapes into the cross-provider [`ChatRequest`] / [`ChatResponse`].
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn id(&self) -> ProviderId;

    /// Human-readable name for the Settings UI.
    fn display_name(&self) -> &str {
        self.id().display_name()
    }

    /// Lightweight liveness check used by the "Test" button. Hits the
    /// provider's cheapest endpoint that requires auth (usually the
    /// models list) and returns Ok if the API key works.
    async fn test(&self) -> Result<()>;

    /// List the models this provider exposes for the configured key.
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;

    /// Run one chat completion. Phase 1 ships non-streaming only.
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
}
