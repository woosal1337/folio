//! Pluggable embedding-provider abstraction. v2 finding 050 / GET-67.
//!
//! Memory retrieval today uses OpenAI's text-embedding-3-large
//! (3072 dims). When the user opts into Privacy Mode (#048), that
//! path is closed; the fallback is a small local ONNX model run
//! via `fastembed-rs` — bge-small-en-v1.5 (384 dims) or
//! nomic-embed-text-v1.5 (768 dims). Lower retrieval fidelity is
//! fine for the "I can still grep my own memory" promise.
//!
//! This module ships the trait + the model-id enum + the
//! provider-id-to-dimension lookup that the SQLite index uses to
//! pick the right vector column. Wiring fastembed-rs lives behind
//! a cargo feature (off by default to keep cold build time low)
//! and arrives in the follow-up.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Identifier the index column writes when persisting a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
#[serde(rename_all = "kebab-case")]
pub enum EmbeddingModel {
    #[serde(rename = "openai-text-embedding-3-large")]
    OpenAi3Large,
    #[serde(rename = "bge-small-en-v1.5")]
    BgeSmallEnV15,
    #[serde(rename = "nomic-embed-text-v1.5")]
    NomicEmbedTextV15,
}

impl EmbeddingModel {
    pub fn id(self) -> &'static str {
        match self {
            EmbeddingModel::OpenAi3Large => "openai-text-embedding-3-large",
            EmbeddingModel::BgeSmallEnV15 => "bge-small-en-v1.5",
            EmbeddingModel::NomicEmbedTextV15 => "nomic-embed-text-v1.5",
        }
    }

    pub fn from_id(id: &str) -> Option<EmbeddingModel> {
        Some(match id {
            "openai-text-embedding-3-large" => EmbeddingModel::OpenAi3Large,
            "bge-small-en-v1.5" => EmbeddingModel::BgeSmallEnV15,
            "nomic-embed-text-v1.5" => EmbeddingModel::NomicEmbedTextV15,
            _ => return None,
        })
    }

    pub fn dimensions(self) -> usize {
        match self {
            EmbeddingModel::OpenAi3Large => 3072,
            EmbeddingModel::BgeSmallEnV15 => 384,
            EmbeddingModel::NomicEmbedTextV15 => 768,
        }
    }

    /// `true` for models that run on-device (no network egress).
    /// Memory retrieval honours Privacy Mode by only picking from
    /// the is_local() set when the toggle is on.
    pub fn is_local(self) -> bool {
        matches!(
            self,
            EmbeddingModel::BgeSmallEnV15 | EmbeddingModel::NomicEmbedTextV15
        )
    }
}

/// Trait the index calls to embed a chunk of text. The OpenAI
/// implementation already exists in
/// `attune-core::memory::embed::EmbeddingClient`; the local
/// implementation arrives behind a cargo feature in the follow-up.
pub trait EmbeddingProvider: Send + Sync {
    fn model(&self) -> EmbeddingModel;
    fn embed(&self, text: &str) -> std::result::Result<Vec<f32>, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_match_known_specs() {
        assert_eq!(EmbeddingModel::OpenAi3Large.dimensions(), 3072);
        assert_eq!(EmbeddingModel::BgeSmallEnV15.dimensions(), 384);
        assert_eq!(EmbeddingModel::NomicEmbedTextV15.dimensions(), 768);
    }

    #[test]
    fn is_local_only_picks_on_device_models() {
        assert!(!EmbeddingModel::OpenAi3Large.is_local());
        assert!(EmbeddingModel::BgeSmallEnV15.is_local());
        assert!(EmbeddingModel::NomicEmbedTextV15.is_local());
    }

    #[test]
    fn id_round_trips() {
        for m in [
            EmbeddingModel::OpenAi3Large,
            EmbeddingModel::BgeSmallEnV15,
            EmbeddingModel::NomicEmbedTextV15,
        ] {
            assert_eq!(EmbeddingModel::from_id(m.id()), Some(m));
        }
        assert!(EmbeddingModel::from_id("does-not-exist").is_none());
    }
}
