use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{FolioError, Result};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    RecordingFinished,
    TranscriptReady,
    TaskCreated,
    MemoryCreated,
}

impl WebhookEvent {
    pub fn as_topic(self) -> &'static str {
        match self {
            WebhookEvent::RecordingFinished => "recording.finished",
            WebhookEvent::TranscriptReady => "transcript.ready",
            WebhookEvent::TaskCreated => "task.created",
            WebhookEvent::MemoryCreated => "memory.created",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct WebhookSubscription {
    pub id: String,

    pub label: String,

    pub url: String,

    pub secret: String,

    #[serde(default)]
    pub events: Vec<WebhookEvent>,

    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub topic: String,
    pub emitted_at: String,
    pub data: serde_json::Value,
}

#[derive(Debug)]
pub struct WebhookStore {
    path: PathBuf,
}

impl WebhookStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default_location() -> Self {
        Self::new(default_webhooks_path())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Vec<WebhookSubscription> {
        let raw = match fs::read_to_string(&self.path) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self, subs: &[WebhookSubscription]) -> Result<()> {
        for sub in subs {
            validate(sub)?;
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                FolioError::Storage(format!(
                    "could not create webhooks dir {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(subs)
            .map_err(|e| FolioError::Storage(format!("webhooks serialize: {e}")))?;
        let mut f = fs::File::create(&tmp)
            .map_err(|e| FolioError::Storage(format!("create webhook tmp: {e}")))?;
        f.write_all(&json)
            .map_err(|e| FolioError::Storage(format!("write webhooks: {e}")))?;
        f.sync_all().ok();
        fs::rename(&tmp, &self.path)
            .map_err(|e| FolioError::Storage(format!("finalize webhooks: {e}")))?;
        Ok(())
    }
}

fn validate(sub: &WebhookSubscription) -> Result<()> {
    if sub.label.trim().is_empty() {
        return Err(FolioError::Storage("webhook label cannot be empty".into()));
    }
    let lower = sub.url.to_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err(FolioError::Storage(format!(
            "webhook url must start with http:// or https://, got {}",
            sub.url
        )));
    }
    if sub.secret.trim().is_empty() {
        return Err(FolioError::Storage("webhook secret cannot be empty".into()));
    }
    Ok(())
}

fn default_webhooks_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    #[cfg(target_os = "macos")]
    {
        home.join("Library")
            .join("Application Support")
            .join("Folio")
            .join("webhooks.json")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".config").join("folio").join("webhooks.json")
    }
}

pub fn new_subscription_id() -> String {
    Uuid::now_v7().to_string()
}

pub fn sign(secret: &str, body: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC-SHA256 accepts any key length");
    mac.update(body);
    let bytes = mac.finalize().into_bytes();
    format!("sha256={}", hex::encode(bytes))
}

pub fn build_payload(event: WebhookEvent, data: serde_json::Value) -> WebhookPayload {
    WebhookPayload {
        topic: event.as_topic().to_string(),
        emitted_at: chrono::Utc::now().to_rfc3339(),
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub() -> WebhookSubscription {
        WebhookSubscription {
            id: new_subscription_id(),
            label: "Notion sync".into(),
            url: "http://localhost:9000/folio".into(),
            secret: "top-secret".into(),
            events: vec![WebhookEvent::RecordingFinished],
            enabled: true,
        }
    }

    #[test]
    fn sign_matches_known_vector() {
        let sig = sign("top-secret", b"hello");
        assert_eq!(
            sig,
            "sha256=e85e6cee68a0c859caec48137a1145ff7d85b8baef24eb4c576cd729e117ab40"
        );
    }

    #[test]
    fn validate_rejects_non_http_urls() {
        let mut bad = sub();
        bad.url = "ftp://nope.example.com".into();
        assert!(validate(&bad).is_err());
    }

    #[test]
    fn validate_rejects_empty_label() {
        let mut bad = sub();
        bad.label = "   ".into();
        assert!(validate(&bad).is_err());
    }

    #[test]
    fn validate_rejects_empty_secret() {
        let mut bad = sub();
        bad.secret = String::new();
        assert!(validate(&bad).is_err());
    }

    #[test]
    fn round_trip_through_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = WebhookStore::new(dir.path().join("hooks.json"));
        let subs = vec![sub()];
        store.save(&subs).unwrap();
        let loaded = store.load();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].label, "Notion sync");
        assert_eq!(loaded[0].events, vec![WebhookEvent::RecordingFinished]);
    }

    #[test]
    fn build_payload_carries_topic_and_data() {
        let p = build_payload(
            WebhookEvent::TaskCreated,
            serde_json::json!({ "title": "Send invoice" }),
        );
        assert_eq!(p.topic, "task.created");
        assert_eq!(p.data["title"], "Send invoice");
    }
}
