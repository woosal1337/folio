//! `/api/referrals/*` — companion to the Settings → Referrals UI.

use serde::Serialize;

use crate::backend::client::{BackendClient, BackendError};
use crate::backend::types::{ReferralRedeemPayload, ReferralStats, ReferralTokenResponse};

#[derive(Serialize)]
struct EmptyBody {}

pub async fn generate_token(client: &BackendClient) -> Result<ReferralTokenResponse, BackendError> {
    client.post("/referrals/generate", &EmptyBody {}).await
}

pub async fn redeem(
    client: &BackendClient,
    token: &str,
    new_user_id: &str,
    new_user_email: &str,
) -> Result<(), BackendError> {
    let payload = ReferralRedeemPayload {
        new_user_id,
        new_user_email,
    };
    let path = format!("/referrals/redeem/{}", token);
    // Endpoint returns `data: { redemption_id, referrer_id }` but the
    // caller only needs the success signal; we discard the body.
    client
        .post::<ReferralRedeemPayload<'_>, serde_json::Value>(&path, &payload)
        .await
        .map(|_| ())
}

pub async fn stats(client: &BackendClient) -> Result<ReferralStats, BackendError> {
    client.get("/referrals/me").await
}
