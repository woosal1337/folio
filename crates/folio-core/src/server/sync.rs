use std::path::Path;

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::error::Result;
use crate::server::client::RemoteClient;
use crate::server::sync_state::{self, RemoteStatus, SyncState, UploadPhase};
use crate::storage::session::TRANSCRIPT_FILENAME;
use crate::transcription::SessionTranscript;

const CHANNELS: &[&str] = &["mic", "system"];

#[derive(Debug, Clone)]
pub struct SyncOutcome {
    pub state: SyncState,
    pub transcript_written: bool,
}

pub async fn sync_session(
    client: &RemoteClient,
    session_dir: &Path,
    language: Option<&str>,
) -> Result<SyncOutcome> {
    let mut state = match sync_state::load(session_dir)? {
        Some(existing) => existing,
        None => SyncState::new(new_recording_id()),
    };

    let label = session_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("session")
        .to_string();

    if state.remote_recording_id.is_none() {
        let duration = CHANNELS
            .iter()
            .map(|c| wav_duration_secs(&session_dir.join(format!("{c}.wav"))))
            .max()
            .unwrap_or(0);
        let recording = client
            .create_recording(&state.recording_id, &label, duration)
            .await?;
        state.remote_recording_id = Some(recording.id);
        sync_state::save(session_dir, &state)?;
    }

    let remote_id = state
        .remote_recording_id
        .clone()
        .expect("remote_recording_id set above");

    if state.upload_state != UploadPhase::Complete {
        state.upload_state = UploadPhase::Uploading;
        state.error = None;
        sync_state::save(session_dir, &state)?;
        for channel in CHANNELS {
            let wav = session_dir.join(format!("{channel}.wav"));
            if !wav.exists() {
                continue;
            }
            let data = std::fs::read(&wav)?;
            let digest = sha256_hex(&data);
            client
                .upload_channel(&remote_id, channel, 0, data, true, Some(&digest))
                .await?;
        }
        state.upload_state = UploadPhase::Complete;
        sync_state::save(session_dir, &state)?;
    }

    if state.remote_job_id.is_none() {
        let job = client
            .enqueue_transcribe(&remote_id, language, false)
            .await?;
        state.remote_job_id = Some(job.id);
        state.remote_status = RemoteStatus::Queued;
        sync_state::save(session_dir, &state)?;
    }

    let job_id = state
        .remote_job_id
        .clone()
        .expect("remote_job_id set above");
    let job = client.poll_job(&job_id).await?;
    state.remote_status = map_status(&job.status);

    let mut transcript_written = false;
    match job.status.as_str() {
        "succeeded" => {
            let transcript: SessionTranscript = client.fetch_transcript(&remote_id).await?;
            transcript.write_json(&session_dir.join(TRANSCRIPT_FILENAME))?;
            state.last_synced_at = Some(Utc::now());
            state.error = None;
            transcript_written = true;
        }
        "failed" => {
            state.error = job.error.clone();
        }
        _ => {}
    }

    sync_state::save(session_dir, &state)?;
    Ok(SyncOutcome {
        state,
        transcript_written,
    })
}

pub fn is_terminal(status: RemoteStatus) -> bool {
    matches!(status, RemoteStatus::Succeeded | RemoteStatus::Failed)
}

fn map_status(status: &str) -> RemoteStatus {
    match status {
        "queued" => RemoteStatus::Queued,
        "running" => RemoteStatus::Running,
        "succeeded" => RemoteStatus::Succeeded,
        "failed" => RemoteStatus::Failed,
        _ => RemoteStatus::None,
    }
}

fn new_recording_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn wav_duration_secs(path: &Path) -> i64 {
    hound::WavReader::open(path)
        .ok()
        .map(|reader| {
            let spec = reader.spec();
            let channels = spec.channels.max(1) as f64;
            let sample_rate = spec.sample_rate.max(1) as f64;
            (reader.len() as f64 / channels / sample_rate) as i64
        })
        .unwrap_or(0)
}
