use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AttuneError, Result};

const INFLIGHT_DIRNAME: &str = "_inflight";

pub fn root(memory_dir: &Path) -> PathBuf {
    memory_dir.join(".attune").join(INFLIGHT_DIRNAME)
}

pub fn session_dir(memory_dir: &Path, session_id: &str) -> PathBuf {
    root(memory_dir).join(session_id)
}

pub fn chunk_path(memory_dir: &Path, session_id: &str, channel: &str, chunk_index: u32) -> PathBuf {
    session_dir(memory_dir, session_id)
        .join(channel)
        .join(format!("{:04}.wav", chunk_index))
}

pub fn ensure_channel_dir(memory_dir: &Path, session_id: &str, channel: &str) -> Result<PathBuf> {
    let path = session_dir(memory_dir, session_id).join(channel);
    fs::create_dir_all(&path).map_err(|e| {
        AttuneError::Storage(format!(
            "create inflight channel dir {}: {e}",
            path.display()
        ))
    })?;
    Ok(path)
}

#[derive(Debug, Clone)]
pub struct InflightSession {
    pub session_id: String,
    pub session_dir: PathBuf,
    pub channels: Vec<InflightChannel>,
}

#[derive(Debug, Clone)]
pub struct InflightChannel {
    pub channel: String,
    pub chunks: Vec<PathBuf>,
}

pub fn list_recoverable(memory_dir: &Path) -> Vec<InflightSession> {
    let r = root(memory_dir);
    let entries = match fs::read_dir(&r) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<InflightSession> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let session_id = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_default();
        let mut channels: Vec<InflightChannel> = Vec::new();
        let channel_entries = match fs::read_dir(&path) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for channel_entry in channel_entries.flatten() {
            let channel_path = channel_entry.path();
            if !channel_path.is_dir() {
                continue;
            }
            let channel = channel_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            let mut chunks: Vec<PathBuf> = fs::read_dir(&channel_path)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("wav"))
                .collect();
            chunks.sort();
            if !chunks.is_empty() {
                channels.push(InflightChannel { channel, chunks });
            }
        }
        if !channels.is_empty() {
            out.push(InflightSession {
                session_id,
                session_dir: path,
                channels,
            });
        }
    }
    out
}

pub fn discard(session: &InflightSession) -> Result<()> {
    fs::remove_dir_all(&session.session_dir).map_err(|e| {
        AttuneError::Storage(format!(
            "remove inflight session {}: {e}",
            session.session_dir.display()
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_recoverable_finds_nothing_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_recoverable(dir.path()).is_empty());
    }

    #[test]
    fn ensure_channel_dir_creates_nested_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = ensure_channel_dir(dir.path(), "sess-1", "mic").unwrap();
        assert!(path.is_dir());
        assert!(path.ends_with(".attune/_inflight/sess-1/mic"));
    }

    #[test]
    fn chunk_path_pads_index_to_four_digits() {
        let dir = tempfile::tempdir().unwrap();
        let path = chunk_path(dir.path(), "sess-1", "mic", 7);
        assert!(path.ends_with("0007.wav"));
    }

    #[test]
    fn list_recoverable_groups_chunks_by_session_and_channel() {
        let dir = tempfile::tempdir().unwrap();
        ensure_channel_dir(dir.path(), "sess-A", "mic").unwrap();
        ensure_channel_dir(dir.path(), "sess-A", "system").unwrap();
        ensure_channel_dir(dir.path(), "sess-B", "mic").unwrap();
        fs::write(chunk_path(dir.path(), "sess-A", "mic", 1), b"WAV").unwrap();
        fs::write(chunk_path(dir.path(), "sess-A", "mic", 2), b"WAV").unwrap();
        fs::write(chunk_path(dir.path(), "sess-A", "system", 1), b"WAV").unwrap();
        fs::write(chunk_path(dir.path(), "sess-B", "mic", 1), b"WAV").unwrap();

        let sessions = list_recoverable(dir.path());
        assert_eq!(sessions.len(), 2);
        let a = sessions.iter().find(|s| s.session_id == "sess-A").unwrap();
        assert_eq!(a.channels.len(), 2);
        let mic = a.channels.iter().find(|c| c.channel == "mic").unwrap();
        assert_eq!(mic.chunks.len(), 2);
    }

    #[test]
    fn discard_removes_the_session_dir() {
        let dir = tempfile::tempdir().unwrap();
        ensure_channel_dir(dir.path(), "sess-X", "mic").unwrap();
        fs::write(chunk_path(dir.path(), "sess-X", "mic", 1), b"WAV").unwrap();
        let sessions = list_recoverable(dir.path());
        let target = sessions
            .into_iter()
            .find(|s| s.session_id == "sess-X")
            .unwrap();
        discard(&target).unwrap();
        assert!(!session_dir(dir.path(), "sess-X").exists());
    }
}
