use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{FolioError, Result};

const AUDIT_DIRNAME: &str = "_audit";
const EGRESS_FILENAME: &str = "egress.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct EgressEntry {
    pub id: String,
    pub emitted_at: String,
    pub endpoint: String,
    pub provider: String,
    pub bytes: u64,

    pub recording_id: String,
    pub cost_usd: f64,

    pub prev_sha256: String,
}

fn path_for(memory_dir: &Path) -> PathBuf {
    memory_dir
        .join(".folio")
        .join(AUDIT_DIRNAME)
        .join(EGRESS_FILENAME)
}

fn last_line_sha(path: &Path) -> Result<String> {
    let f = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(String::new()),
        Err(e) => {
            return Err(FolioError::Storage(format!(
                "open egress log {}: {e}",
                path.display()
            )))
        }
    };
    let mut last = String::new();
    for line in BufReader::new(f).lines().map_while(std::result::Result::ok) {
        if !line.trim().is_empty() {
            last = line;
        }
    }
    if last.is_empty() {
        return Ok(String::new());
    }
    let mut hasher = Sha256::new();
    hasher.update(last.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

#[allow(clippy::too_many_arguments)]
pub fn append(
    memory_dir: &Path,
    endpoint: impl Into<String>,
    provider: impl Into<String>,
    bytes: u64,
    recording_id: impl Into<String>,
    cost_usd: f64,
) -> Result<EgressEntry> {
    let path = path_for(memory_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            FolioError::Storage(format!("create_dir_all {}: {e}", parent.display()))
        })?;
    }
    let entry = EgressEntry {
        id: Uuid::now_v7().to_string(),
        emitted_at: Utc::now().to_rfc3339(),
        endpoint: endpoint.into(),
        provider: provider.into(),
        bytes,
        recording_id: recording_id.into(),
        cost_usd,
        prev_sha256: last_line_sha(&path)?,
    };
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| FolioError::Storage(format!("open egress log {}: {e}", path.display())))?;
    let line = serde_json::to_string(&entry)
        .map_err(|e| FolioError::Storage(format!("egress serialize: {e}")))?;
    writeln!(file, "{line}")
        .map_err(|e| FolioError::Storage(format!("write egress log {}: {e}", path.display())))?;
    Ok(entry)
}

pub fn read_all(memory_dir: &Path) -> Vec<EgressEntry> {
    let path = path_for(memory_dir);
    let f = match fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for line in BufReader::new(f).lines().map_while(std::result::Result::ok) {
        if let Ok(entry) = serde_json::from_str::<EgressEntry>(&line) {
            out.push(entry);
        }
    }
    out
}

pub fn verify_chain(memory_dir: &Path) -> Option<usize> {
    let path = path_for(memory_dir);
    let f = fs::File::open(&path).ok()?;
    let mut prev_hash = String::new();
    for (idx, line) in BufReader::new(f).lines().enumerate() {
        let Ok(line) = line else {
            return Some(idx + 1);
        };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(entry) = serde_json::from_str::<EgressEntry>(&line) else {
            return Some(idx + 1);
        };
        if entry.prev_sha256 != prev_hash {
            return Some(idx + 1);
        }
        let mut h = Sha256::new();
        h.update(line.as_bytes());
        prev_hash = format!("{:x}", h.finalize());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_creates_chain_from_scratch() {
        let dir = tempfile::tempdir().unwrap();
        let first = append(dir.path(), "openai/whisper", "openai", 1024, "rec1", 0.06).unwrap();
        assert!(first.prev_sha256.is_empty());
        let second = append(dir.path(), "openai/chat", "openai", 2048, "rec1", 0.12).unwrap();
        assert!(!second.prev_sha256.is_empty());
        let entries = read_all(dir.path());
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn verify_chain_returns_none_when_intact() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..3 {
            append(dir.path(), format!("call/{i}"), "openai", 100, "rec", 0.01).unwrap();
        }
        assert!(verify_chain(dir.path()).is_none());
    }

    #[test]
    fn verify_chain_flags_a_torn_out_line() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..3 {
            append(dir.path(), format!("call/{i}"), "openai", 100, "rec", 0.01).unwrap();
        }
        let path = path_for(dir.path());
        let lines: Vec<String> = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(String::from)
            .collect();

        let damaged = format!("{}\n{}\n", lines[0], lines[2]);
        fs::write(&path, damaged).unwrap();
        let bad = verify_chain(dir.path()).expect("chain should be broken");
        assert_eq!(bad, 2);
    }
}
