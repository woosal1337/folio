use attune_core::webhooks::{
    build_payload, new_subscription_id, sign, WebhookEvent, WebhookStore, WebhookSubscription,
};
use serde_json::Value as JsonValue;
use tauri::async_runtime::spawn_blocking;
use tracing::{info, warn};

fn store() -> WebhookStore {
    WebhookStore::default_location()
}

#[tauri::command]
pub async fn list_webhooks() -> Result<Vec<WebhookSubscription>, String> {
    spawn_blocking(|| store().load())
        .await
        .map_err(|e| format!("list_webhooks task panicked: {e}"))
}

#[tauri::command]
pub async fn save_webhook(
    mut subscription: WebhookSubscription,
) -> Result<WebhookSubscription, String> {
    if subscription.id.trim().is_empty() {
        subscription.id = new_subscription_id();
    }
    spawn_blocking(move || -> Result<WebhookSubscription, String> {
        let s = store();
        let mut subs = s.load();
        if let Some(existing) = subs.iter_mut().find(|w| w.id == subscription.id) {
            *existing = subscription.clone();
        } else {
            subs.push(subscription.clone());
        }
        s.save(&subs).map_err(|e| e.to_string())?;
        Ok(subscription)
    })
    .await
    .map_err(|e| format!("save_webhook task panicked: {e}"))?
}

#[tauri::command]
pub async fn delete_webhook(id: String) -> Result<(), String> {
    spawn_blocking(move || -> Result<(), String> {
        let s = store();
        let mut subs = s.load();
        subs.retain(|w| w.id != id);
        s.save(&subs).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("delete_webhook task panicked: {e}"))?
}

#[tauri::command]
pub async fn test_webhook(id: String) -> Result<String, String> {
    let sub = spawn_blocking(move || -> Option<WebhookSubscription> {
        store().load().into_iter().find(|w| w.id == id)
    })
    .await
    .map_err(|e| format!("test_webhook task panicked: {e}"))?;
    let sub = sub.ok_or_else(|| "webhook subscription not found".to_string())?;

    let payload = build_payload(
        WebhookEvent::RecordingFinished,
        serde_json::json!({
            "test": true,
            "message": "Test event from Settings → Webhooks",
        }),
    );
    deliver(&sub, &payload)
        .await
        .map_err(|e| format!("test webhook failed: {e}"))
}

#[allow(dead_code)]
pub async fn dispatch(event: WebhookEvent, data: JsonValue) {
    let subs = match spawn_blocking(|| store().load()).await {
        Ok(s) => s,
        Err(e) => {
            warn!(error = %e, "could not load webhook subscriptions for dispatch");
            return;
        }
    };
    if subs.is_empty() {
        return;
    }
    let payload = build_payload(event, data);
    for sub in subs {
        if !sub.enabled {
            continue;
        }
        if !sub.events.is_empty() && !sub.events.contains(&event) {
            continue;
        }
        let payload = payload.clone();
        let label = sub.label.clone();
        match deliver(&sub, &payload).await {
            Ok(status) => {
                info!(target: "attune::webhooks", %label, status, topic = %payload.topic, "delivered")
            }
            Err(e) => {
                warn!(target: "attune::webhooks", %label, topic = %payload.topic, error = %e, "delivery failed")
            }
        }
    }
}

async fn deliver(
    sub: &WebhookSubscription,
    payload: &attune_core::webhooks::WebhookPayload,
) -> Result<String, String> {
    let host = attune_core::cloud_guard::host_of(&sub.url).unwrap_or_default();
    attune_core::cloud_guard::ensure_allowed(host).map_err(|e| e.to_string())?;

    let body = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
    let signature = sign(&sub.secret, &body);
    let client = reqwest::Client::new();
    let resp = client
        .post(&sub.url)
        .header("Content-Type", "application/json")
        .header("X-Attune-Signature", signature)
        .header("X-Attune-Topic", &payload.topic)
        .body(body)
        .send()
        .await
        .map_err(|e| redact_reqwest_error(&e, &sub.url))?;
    Ok(resp.status().to_string())
}

fn redact_reqwest_error(error: &reqwest::Error, full_url: &str) -> String {
    let host = url::Url::parse(full_url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_else(|| "<unknown host>".to_string());
    let message = error.to_string().replace(full_url, &host);
    format!("webhook delivery to {host} failed: {message}")
}
