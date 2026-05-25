//! OpenAI embeddings client for the memory layer.
//!
//! One model only: `text-embedding-3-large` (3072 dims). Cost is
//! $0.00013 per 1K tokens — for a typical memory string of <50
//! tokens, ~$0.000007 per write. Negligible at human scale.
//!
//! We don't model "embedding providers" the way the chat layer does
//! because the memory index schema pins a single vector dimensionality
//! and we'd rather keep one provider than build the indirection.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::{AttuneError, Result};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const MODEL: &str = "text-embedding-3-large";

/// Embedding client. Cheap to construct; holds a single
/// reqwest::Client.
pub struct EmbeddingClient {
    api_key: String,
    base_url: String,
    client: Client,
}

impl EmbeddingClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_base_url(api_key, DEFAULT_BASE_URL)
    }

    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: Client::new(),
        }
    }

    /// Embed a single string. Returns a 3072-dim f32 vector.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/embeddings", self.base_url);
        let body = EmbeddingsRequest {
            model: MODEL.to_string(),
            input: text.to_string(),
            encoding_format: "float".to_string(),
        };
        debug!(chars = text.len(), "openai embeddings request");
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AttuneError::Llm(format!("embeddings request failed: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AttuneError::Llm(format!(
                "embeddings returned HTTP {status}: {body}"
            )));
        }
        let parsed: EmbeddingsResponse = resp
            .json()
            .await
            .map_err(|e| AttuneError::Llm(format!("embeddings json decode failed: {e}")))?;
        let item = parsed
            .data
            .into_iter()
            .next()
            .ok_or_else(|| AttuneError::Llm("embeddings returned zero items".to_string()))?;
        Ok(item.embedding)
    }
}

#[derive(Serialize)]
struct EmbeddingsRequest {
    model: String,
    input: String,
    encoding_format: String,
}

#[derive(Deserialize)]
struct EmbeddingsResponse {
    data: Vec<EmbeddingItem>,
}

#[derive(Deserialize)]
struct EmbeddingItem {
    embedding: Vec<f32>,
}
