use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default)]
    pub token_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateRecordingRequest {
    pub client_id: String,
    pub label: String,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChannelInfo {
    pub name: String,
    #[serde(default)]
    pub size_bytes: i64,
    #[serde(default)]
    pub upload_complete: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteRecording {
    pub id: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub duration_seconds: i64,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub channels: Vec<ChannelInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UploadResult {
    pub offset: i64,
    pub complete: bool,
}

#[derive(Debug, Clone)]
pub enum ChunkOutcome {
    Accepted(UploadResult),
    OffsetMismatch { expected: u64 },
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscribeRequest {
    pub language: Option<String>,
    pub diarize: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JobInfo {
    pub id: String,
    #[serde(default)]
    pub recording_id: String,
    pub status: String,
    #[serde(default)]
    pub progress: f64,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub engine: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub gpu: bool,
    #[serde(default)]
    pub diarization: bool,
}
