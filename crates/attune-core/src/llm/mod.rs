//! LLM provider abstraction for the AI chat feature.
//!
//! The architecture lives in the vault at
//! `~/Documents/GitHub/obsidian.md/projects/attune/plan/ai-chat-multi-provider.md`.
//! This module is the Rust embodiment of phase 1: the [`LlmProvider`]
//! trait, an [`OpenAiProvider`] concrete implementation, and a
//! macOS-Keychain-backed [`KeyStore`] for API keys.
//!
//! Phase 1 ships non-streaming chat only — `chat()`. Streaming is added
//! in phase 5; the trait already declares `chat_stream()` as a future
//! contract, marked `#[allow(dead_code)]` until then.
//!
//! Anthropic and DeepSeek arrive in phase 2 and share the same trait.

pub mod agent_run;
pub mod agent_toml;
pub mod agent_tools;
pub mod agents;
pub mod confidence;
pub mod keystore;
pub mod live_agent;
pub mod local_llm;
pub mod marketplace;
pub mod prompt;
pub mod provider;
pub mod providers;
pub mod rate_limit;
pub mod router;
pub mod run_card;
pub mod skills;
pub mod templates;
pub mod types;

pub use agent_run::{AgentRun, AgentRunStore};
pub use agents::Agent;
pub use keystore::KeyStore;
pub use provider::{LlmProvider, ProviderId};
pub use providers::openai::OpenAiProvider;
pub use types::{
    ChatMessage, ChatRequest, ChatResponse, ChatRole, FinishReason, ModelInfo, ProviderConfig,
    ProviderStatus, ToolCall, ToolDef,
};
