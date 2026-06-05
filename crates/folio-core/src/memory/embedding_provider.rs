use serde::{Deserialize, Serialize};
use ts_rs::TS;

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

    pub fn is_local(self) -> bool {
        matches!(
            self,
            EmbeddingModel::BgeSmallEnV15 | EmbeddingModel::NomicEmbedTextV15
        )
    }
}

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
