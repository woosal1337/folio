use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Instant;

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::error::{FolioError, Result};
use crate::server::client::RemoteClient;
use crate::server::sync_state::{self, RemoteStatus, SyncState, UploadPhase};
use crate::server::types::ChunkOutcome;
use crate::storage::session::TRANSCRIPT_FILENAME;
use crate::transcription::SessionTranscript;

const CHANNELS: &[&str] = &["mic", "system"];

const UPLOAD_CHUNK_BYTES: usize = 8 * 1024 * 1024;

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
            upload_channel_chunked(client, &remote_id, channel, &wav).await?;
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

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

async fn upload_channel_chunked(
    client: &RemoteClient,
    remote_id: &str,
    channel: &str,
    wav: &Path,
) -> Result<()> {
    let total = std::fs::metadata(wav)?.len();
    let hash_started = Instant::now();
    let digest = sha256_file(wav)?;
    tracing::info!(
        channel,
        bytes = total,
        hash_secs = hash_started.elapsed().as_secs_f64(),
        "upload: hashed channel"
    );
    let started = Instant::now();
    let mut file = File::open(wav)?;
    let mut offset: u64 = 0;
    let mut stalls = 0u32;
    let mut chunks = 0u32;

    loop {
        let sent_from = offset;
        file.seek(SeekFrom::Start(offset))?;
        let want = std::cmp::min(UPLOAD_CHUNK_BYTES as u64, total.saturating_sub(offset)) as usize;
        let mut buf = vec![0u8; want];
        file.read_exact(&mut buf)?;
        let complete = offset + want as u64 >= total;

        let outcome = client
            .upload_channel_chunk(
                remote_id,
                channel,
                offset,
                buf,
                complete,
                if complete {
                    Some(digest.as_str())
                } else {
                    None
                },
            )
            .await?;

        chunks += 1;
        match outcome {
            ChunkOutcome::Accepted(result) => {
                offset = result.offset.max(0) as u64;
                if result.complete || offset >= total {
                    let secs = started.elapsed().as_secs_f64();
                    tracing::info!(
                        channel,
                        bytes = total,
                        chunks,
                        secs,
                        mb_per_sec = (total as f64 / 1_048_576.0) / secs.max(0.001),
                        "upload: channel complete"
                    );
                    return Ok(());
                }
            }
            ChunkOutcome::OffsetMismatch { expected } => {
                if expected > total {
                    return Err(FolioError::Backend(format!(
                        "upload_channel: server holds {expected} bytes for {channel}, local file is {total}"
                    )));
                }
                offset = expected;
            }
        }

        if offset <= sent_from {
            stalls += 1;
            if stalls > 3 {
                return Err(FolioError::Backend(format!(
                    "upload_channel: {channel} stalled at offset {offset} of {total}"
                )));
            }
        } else {
            stalls = 0;
        }
    }
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
