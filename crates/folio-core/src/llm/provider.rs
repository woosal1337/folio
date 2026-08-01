use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::Result;
use crate::llm::types::{ChatRequest, ChatResponse, ModelInfo};

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/shared/types/")]
pub enum ProviderId {
    OpenAi,
}

impl ProviderId {
    pub fn all() -> &'static [ProviderId] {
        &[ProviderId::OpenAi]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ProviderId::OpenAi => "openai",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            ProviderId::OpenAi => "OpenAI",
        }
    }
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn id(&self) -> ProviderId;

    fn display_name(&self) -> &str {
        self.id().display_name()
    }

    async fn test(&self) -> Result<()>;

    async fn list_models(&self) -> Result<Vec<ModelInfo>>;

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
}
