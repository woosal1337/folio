use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::{FolioError, Result};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const MODEL: &str = "text-embedding-3-large";

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
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            api_key: api_key.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
        }
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/embeddings", self.base_url);
        let host = crate::cloud_guard::host_of(&url).unwrap_or_default();
        crate::cloud_guard::ensure_allowed(host).map_err(|e| FolioError::Llm(e.to_string()))?;
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
            .map_err(|e| FolioError::Llm(format!("embeddings request failed: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(FolioError::Llm(format!(
                "embeddings returned HTTP {status}: {body}"
            )));
        }
        let parsed: EmbeddingsResponse = resp
            .json()
            .await
            .map_err(|e| FolioError::Llm(format!("embeddings json decode failed: {e}")))?;
        let item = parsed
            .data
            .into_iter()
            .next()
            .ok_or_else(|| FolioError::Llm("embeddings returned zero items".to_string()))?;
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

#[cfg(test)]
mod egress_guard_tests {
    use super::*;
    use crate::cloud_guard;

    #[test]
    fn embeddings_url_resolves_to_openai_host() {
        let client = EmbeddingClient::new("sk-test-not-real");
        let url = format!("{}/embeddings", client.base_url);
        let host = cloud_guard::host_of(&url).expect("host must parse");
        assert_eq!(host, "api.openai.com");
    }
}
