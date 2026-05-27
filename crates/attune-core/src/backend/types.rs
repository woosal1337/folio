//! Wire envelope + request/response shapes shared across endpoint modules.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Standard envelope every attune-api endpoint returns. The `data`
/// field is a typed payload per-endpoint; the client unwraps it via
/// [`Envelope::into_data`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub success: bool,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub data: Option<T>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Mirror of `ErrorResponse` in attune-api. Some endpoints return this
/// shape directly when `success: false`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignupRequest<'a> {
    pub email: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodeVerifyRequest<'a> {
    pub email: &'a str,
    pub code: &'a str,
    pub device_id: &'a str,
    pub device_name: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshRequest<'a> {
    pub refresh_token: &'a str,
    pub device_id: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogoutRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<&'a str>,
}

/// `verify_signin_code` returns this — a wrapping `data.user` + tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default)]
    pub user: Option<UserDoc>,
    #[serde(default)]
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
}

/// Server-side user document (the parts the client cares about).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDoc {
    #[serde(rename = "_id", alias = "id")]
    pub id: String,
    pub email: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub privacy_tier: Option<String>,
    #[serde(default)]
    pub subscription_tier: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountUpdateRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDoc {
    pub device_id: String,
    pub device_name: String,
    pub created_at: String,
    #[serde(default)]
    pub last_seen_at: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReferralRedeemPayload<'a> {
    pub new_user_id: &'a str,
    pub new_user_email: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralTokenResponse {
    pub token: String,
    pub share_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralStats {
    pub token: String,
    pub share_url: String,
    pub qualified_count: u32,
    pub pending_count: u32,
    pub free_months_earned: u32,
    pub yearly_cap: u32,
    pub yearly_remaining: u32,
}
