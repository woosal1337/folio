use std::time::Duration;

use reqwest::Client;

use crate::error::{FolioError, Result};
use crate::server::types::{
    Capabilities, CreateRecordingRequest, JobInfo, LoginRequest, RefreshRequest, RegisterRequest,
    RemoteRecording, TokenPair, TranscribeRequest, UploadResult, UserInfo,
};
use crate::transcription::SessionTranscript;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(600);

pub struct RemoteClient {
    base_url: String,
    token: Option<String>,
    client: Client,
}

impl RemoteClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| FolioError::Backend(format!("could not build HTTP client: {e}")))?;
        Ok(Self {
            base_url: normalize(base_url.into()),
            token: None,
            client,
        })
    }

    #[must_use]
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn ensure_egress(&self) -> Result<()> {
        let host = crate::cloud_guard::host_of(&self.base_url).unwrap_or_default();
        crate::cloud_guard::ensure_allowed(host).map_err(|e| FolioError::Backend(e.to_string()))
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn authed(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.token {
            Some(t) => rb.bearer_auth(t),
            None => rb,
        }
    }

    pub async fn capabilities(&self) -> Result<Capabilities> {
        self.ensure_egress()?;
        let resp = self
            .client
            .get(self.url("/v1/capabilities"))
            .send()
            .await
            .map_err(|e| FolioError::Backend(format!("capabilities request failed: {e}")))?;
        decode(resp, "capabilities").await
    }

    pub async fn register(&self, email: &str, password: &str) -> Result<TokenPair> {
        self.post_json(
            "/v1/auth/register",
            &RegisterRequest {
                email: email.to_string(),
                password: password.to_string(),
            },
            "register",
            false,
        )
        .await
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<TokenPair> {
        self.post_json(
            "/v1/auth/login",
            &LoginRequest {
                email: email.to_string(),
                password: password.to_string(),
            },
            "login",
            false,
        )
        .await
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<TokenPair> {
        self.post_json(
            "/v1/auth/refresh",
            &RefreshRequest {
                refresh_token: refresh_token.to_string(),
            },
            "refresh",
            false,
        )
        .await
    }

    pub async fn me(&self) -> Result<UserInfo> {
        self.ensure_egress()?;
        let resp = self
            .authed(self.client.get(self.url("/v1/auth/me")))
            .send()
            .await
            .map_err(|e| FolioError::Backend(format!("me request failed: {e}")))?;
        decode(resp, "me").await
    }

    pub async fn create_recording(
        &self,
        client_id: &str,
        label: &str,
        duration_seconds: i64,
    ) -> Result<RemoteRecording> {
        self.post_json(
            "/v1/recordings",
            &CreateRecordingRequest {
                client_id: client_id.to_string(),
                label: label.to_string(),
                duration_seconds,
            },
            "create_recording",
            true,
        )
        .await
    }

    pub async fn upload_channel(
        &self,
        recording_id: &str,
        channel: &str,
        offset: u64,
        data: Vec<u8>,
        complete: bool,
        sha256: Option<&str>,
    ) -> Result<UploadResult> {
        self.ensure_egress()?;
        let mut rb = self
            .client
            .put(self.url(&format!("/v1/recordings/{recording_id}/channels/{channel}")))
            .header("Upload-Offset", offset.to_string())
            .header("Upload-Complete", if complete { "true" } else { "false" });
        if let Some(s) = sha256 {
            rb = rb.header("X-Content-Sha256", s);
        }
        let resp = self
            .authed(rb)
            .body(data)
            .send()
            .await
            .map_err(|e| FolioError::Backend(format!("upload_channel request failed: {e}")))?;
        decode(resp, "upload_channel").await
    }

    pub async fn enqueue_transcribe(
        &self,
        recording_id: &str,
        language: Option<&str>,
        diarize: bool,
    ) -> Result<JobInfo> {
        self.post_json(
            &format!("/v1/recordings/{recording_id}/transcribe"),
            &TranscribeRequest {
                language: language.map(|s| s.to_string()),
                diarize,
            },
            "enqueue_transcribe",
            true,
        )
        .await
    }

    pub async fn poll_job(&self, job_id: &str) -> Result<JobInfo> {
        self.ensure_egress()?;
        let resp = self
            .authed(self.client.get(self.url(&format!("/v1/jobs/{job_id}"))))
            .send()
            .await
            .map_err(|e| FolioError::Backend(format!("poll_job request failed: {e}")))?;
        decode(resp, "poll_job").await
    }

    pub async fn fetch_transcript(&self, recording_id: &str) -> Result<SessionTranscript> {
        self.ensure_egress()?;
        let resp = self
            .authed(
                self.client
                    .get(self.url(&format!("/v1/recordings/{recording_id}/transcript"))),
            )
            .send()
            .await
            .map_err(|e| FolioError::Backend(format!("fetch_transcript request failed: {e}")))?;
        decode(resp, "fetch_transcript").await
    }

    pub async fn list_recordings(
        &self,
        updated_since: Option<&str>,
    ) -> Result<Vec<RemoteRecording>> {
        self.ensure_egress()?;
        let url = match updated_since {
            Some(since) => format!("{}?updated_since={since}", self.url("/v1/recordings")),
            None => self.url("/v1/recordings"),
        };
        let resp = self
            .authed(self.client.get(url))
            .send()
            .await
            .map_err(|e| FolioError::Backend(format!("list_recordings request failed: {e}")))?;
        decode(resp, "list_recordings").await
    }

    async fn post_json<B, T>(&self, path: &str, body: &B, ctx: &str, auth: bool) -> Result<T>
    where
        B: serde::Serialize,
        T: serde::de::DeserializeOwned,
    {
        self.ensure_egress()?;
        let mut rb = self.client.post(self.url(path)).json(body);
        if auth {
            rb = self.authed(rb);
        }
        let resp = rb
            .send()
            .await
            .map_err(|e| FolioError::Backend(format!("{ctx} request failed: {e}")))?;
        decode(resp, ctx).await
    }
}

fn normalize(url: String) -> String {
    url.trim().trim_end_matches('/').to_string()
}

async fn decode<T: serde::de::DeserializeOwned>(resp: reqwest::Response, ctx: &str) -> Result<T> {
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(FolioError::Backend(format!(
            "{ctx}: HTTP {status}: {}",
            truncate(&body, 400)
        )));
    }
    resp.json::<T>()
        .await
        .map_err(|e| FolioError::Backend(format!("{ctx}: response decode failed: {e}")))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let taken: String = s.chars().take(max).collect();
        format!("{taken}…")
    }
}
