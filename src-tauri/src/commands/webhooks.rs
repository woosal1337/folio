//! Tauri commands for the webhook subscription store.
//!
//! The store lives at the platform's standard config location and is
//! read on every command — webhooks are infrequently edited and the
//! file is tiny, so we don't bother caching in AppState. Posting to
//! subscribers on lifecycle events is handled out-of-band by the
//! existing transcribe + agent pipelines via `dispatch_webhook`.
//!
//! v2 finding 079 / GET-101.

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

/// One-off test POST to a subscription with a fixed payload so the
/// user can verify the endpoint + secret without waiting for a real
/// recording lifecycle event. Returns the HTTP status string the
/// remote returned, or an error if the request failed.
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

/// Best-effort fanout to every enabled subscription that has opted
/// into the given event. Failures are logged at WARN and otherwise
/// silent — webhook delivery is a sync convenience, never the source
/// of truth.
///
/// Currently exported for use by the follow-up PR that wires it into
/// the transcribe / agent / memory post-success paths. Marked
/// `allow(dead_code)` so the receiving side can ship today without a
/// downstream call site.
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
        .map_err(|e| e.to_string())?;
    Ok(resp.status().to_string())
}
