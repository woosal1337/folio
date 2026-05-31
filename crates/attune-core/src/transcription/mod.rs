//! Speech-to-text transcription backends.
//!
//! [`Transcriber`] is the trait the rest of the app talks to. Concrete
//! implementations live in submodules: [`openai`] for the hosted Whisper
//! API and [`stub`] for tests and offline use.

pub mod adaptive;
pub mod chunker;
pub mod hallucination_filter;
pub mod local;
pub mod locate;
pub mod model_lru;
pub mod models;
pub mod openai;
pub mod stub;
pub mod upload_state;
pub mod vad;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use local::LocalWhisperTranscriber;
pub use models::{DownloadProgress, WhisperModel, WhisperModelStatus, WhisperModelStore};
pub use openai::OpenAiTranscriber;
pub use stub::StubTranscriber;

use crate::error::{AttuneError, Result};
use crate::storage::atomic_write::atomic_write;

/// A timestamped slice of recognised speech.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct TranscriptSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub text: String,
    /// Diarized speaker index for this segment (GET-189). `None` until
    /// diarization runs; set on the system channel (the mic channel is
    /// the user by definition). Raw cluster indices from the diarizer;
    /// the UI relabels them "Speaker 1/2/3…" by order of appearance.
    #[serde(default)]
    pub speaker: Option<i32>,
}

/// A full transcript for a single audio channel: the ordered sequence
/// of segments and the language the backend identified. This is the
/// shape each individual whisper run produces, before we attach a
/// channel name to it.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct Transcript {
    pub language: Option<String>,
    pub segments: Vec<TranscriptSegment>,
}

/// One channel's transcript inside a [`SessionTranscript`]. The
/// `channel` field is the same identifier used elsewhere ("mic" /
/// "system") so the UI can label it ("You" / "Others") consistently.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ChannelTranscript {
    /// "mic" or "system" — the audio channel this transcript was
    /// produced from.
    pub channel: String,
    pub language: Option<String>,
    pub segments: Vec<TranscriptSegment>,
}

/// Full per-session transcript: one [`ChannelTranscript`] per audio
/// channel that produced output. This is the shape that lives in
/// `<session_dir>/transcript.json` and the shape the frontend reads
/// and edits.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct SessionTranscript {
    pub channels: Vec<ChannelTranscript>,
}

/// Outcome of a transcription run: the per-channel transcripts plus
/// the session they belong to and the on-disk JSON they were
/// persisted to.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct TranscriptionResult {
    pub session_dir: std::path::PathBuf,
    pub transcript_path: std::path::PathBuf,
    pub session_transcript: SessionTranscript,
}

impl Transcript {
    pub fn full_text(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Zstd compression level for new transcripts. Level 3 is the
/// library default — a good balance between compression ratio (3x
/// on typical pretty-printed JSON) and write latency. v2 finding 066.
const ZSTD_LEVEL: i32 = 3;

/// Suffix appended to the canonical transcript filename when the
/// file is zstd-compressed.
const ZSTD_SUFFIX: &str = ".zst";

impl SessionTranscript {
    /// Persist the transcript bundle as zstd-compressed pretty-printed
    /// JSON. The file written has the `.zst` suffix appended to `path`
    /// (e.g. `transcript.json` → `transcript.json.zst`). New writes
    /// always compress; legacy uncompressed transcripts continue to be
    /// readable by [`Self::read_json`] for back-compat, and the legacy
    /// uncompressed file is removed once the compressed one has landed
    /// atomically.
    pub fn write_json(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            AttuneError::Transcription(format!("could not serialize transcript: {e}"))
        })?;
        let compressed = zstd::encode_all(json.as_bytes(), ZSTD_LEVEL)
            .map_err(|e| AttuneError::Transcription(format!("zstd compress failed: {e}")))?;
        let zst_path = path.with_extension(format!(
            "{}{ZSTD_SUFFIX}",
            path.extension().and_then(|e| e.to_str()).unwrap_or("json")
        ));
        atomic_write(&zst_path, &compressed)?;
        // Best-effort cleanup of the legacy uncompressed file once the
        // compressed one is on disk. Missing-file is success.
        if zst_path != path && path.exists() {
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }

    /// Read a transcript from disk.
    ///
    /// Reader resolution order:
    ///   1. `path` exactly as given (back-compat with uncompressed
    ///      `transcript.json`)
    ///   2. `path + ".zst"` (the new default)
    ///   3. If the bytes start with the zstd magic header, decompress
    ///      regardless of the file extension (handles renamed files)
    ///
    /// Older transcripts (single-channel, no `channels` field) are
    /// detected and lifted into the new shape with the channel labelled
    /// `"legacy"` so existing transcripts keep working through the UI
    /// without forcing the user to re-transcribe.
    pub fn read_json(path: &Path) -> Result<Self> {
        let raw = read_transcript_bytes(path)?;

        if let Ok(session) = serde_json::from_str::<SessionTranscript>(&raw) {
            return Ok(session);
        }

        // Pre-multichannel transcripts were just `{ language, segments }`
        // at the top level. Lift them into the new shape with a single
        // legacy channel.
        let legacy: Transcript = serde_json::from_str(&raw).map_err(|e| {
            AttuneError::Transcription(format!(
                "could not parse transcript {}: {e}",
                path.display()
            ))
        })?;
        Ok(SessionTranscript {
            channels: vec![ChannelTranscript {
                channel: "legacy".to_string(),
                language: legacy.language,
                segments: legacy.segments,
            }],
        })
    }

    /// Render the whole session as a single chronological, speaker-
    /// labelled dialogue — the form the LLM agents (and any "who said
    /// what" consumer) read. Segments from every channel are merged and
    /// sorted by start time, each prefixed with its speaker:
    ///
    /// - the mic channel is the note-taker → `You`
    /// - system speakers become `Speaker 1`, `Speaker 2`, … numbered by
    ///   first appearance (matching the transcript UI's labelling)
    /// - un-diarized system audio falls back to `Others`
    ///
    /// Empty segments are dropped. With `with_timestamps`, each line is
    /// prefixed `[mm:ss] ` so the model can cite moments.
    ///
    /// GET-189: speaker labels make the AI's attribution ("Speaker 2
    /// committed to…") precise instead of lumping everyone into "Others".
    pub fn to_labeled_dialogue(&self, with_timestamps: bool) -> String {
        self.to_labeled_dialogue_named(with_timestamps, &std::collections::HashMap::new())
    }

    /// Like [`Self::to_labeled_dialogue`], but a `names` map (raw diarizer
    /// cluster id → display name, from the session's speaker sidecar)
    /// overrides the generic `Speaker N` label with the real name the user
    /// gave that voice. A cluster without a name still falls back to
    /// `Speaker N` (numbered by first appearance), so the two never clash.
    pub fn to_labeled_dialogue_named(
        &self,
        with_timestamps: bool,
        names: &std::collections::HashMap<i32, String>,
    ) -> String {
        use std::collections::HashMap;

        // 1-based display number per raw diarizer cluster index, by first
        // appearance in the system channel (same ordering the UI uses).
        let mut speaker_num: HashMap<i32, usize> = HashMap::new();
        for ch in &self.channels {
            if ch.channel == "system" {
                for seg in &ch.segments {
                    if let Some(spk) = seg.speaker {
                        let next = speaker_num.len() + 1;
                        speaker_num.entry(spk).or_insert(next);
                    }
                }
            }
        }

        let mut lines: Vec<(f64, String, &str)> = Vec::new();
        for ch in &self.channels {
            for seg in &ch.segments {
                let text = seg.text.trim();
                if text.is_empty() {
                    continue;
                }
                let label = match ch.channel.as_str() {
                    "mic" => "You".to_string(),
                    "system" => match seg.speaker {
                        Some(spk) => names.get(&spk).cloned().unwrap_or_else(|| {
                            format!("Speaker {}", speaker_num.get(&spk).copied().unwrap_or(0))
                        }),
                        None => "Others".to_string(),
                    },
                    "legacy" => "Unknown speaker".to_string(),
                    other => other.to_string(),
                };
                lines.push((seg.start_seconds, label, text));
            }
        }
        lines.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut out = String::new();
        for (start, label, text) in lines {
            if with_timestamps {
                out.push_str(&format!("[{}] {}: {}\n", fmt_mmss(start), label, text));
            } else {
                out.push_str(&format!("{}: {}\n", label, text));
            }
        }
        out.trim().to_string()
    }
}

/// Format seconds as `m:ss` (or `h:mm:ss` past an hour) for transcript
/// lines.
fn fmt_mmss(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// First four bytes of every zstd-compressed frame
/// (`0xFD2FB528`, little-endian). Used to sniff whether a transcript
/// blob is compressed independent of its filename so the reader can
/// recover from files with the wrong extension.
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

/// Resolve and return the transcript bytes as a UTF-8 String. Tries
/// the uncompressed path first (back-compat), then the same path with
/// `.zst` appended, then falls back to magic-byte sniffing.
fn read_transcript_bytes(path: &Path) -> Result<String> {
    let candidates: [PathBuf; 2] = [
        path.to_path_buf(),
        path.with_extension(format!(
            "{}{ZSTD_SUFFIX}",
            path.extension().and_then(|e| e.to_str()).unwrap_or("json")
        )),
    ];
    for candidate in &candidates {
        let Ok(bytes) = std::fs::read(candidate) else {
            continue;
        };
        let is_compressed = candidate
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "zst")
            .unwrap_or(false)
            || bytes.starts_with(&ZSTD_MAGIC);
        let payload = if is_compressed {
            zstd::decode_all(bytes.as_slice()).map_err(|e| {
                AttuneError::Transcription(format!(
                    "zstd decompress {} failed: {e}",
                    candidate.display()
                ))
            })?
        } else {
            bytes
        };
        return String::from_utf8(payload).map_err(|e| {
            AttuneError::Transcription(format!(
                "transcript {} is not UTF-8: {e}",
                candidate.display()
            ))
        });
    }
    Err(AttuneError::Transcription(format!(
        "could not find transcript at {} or {}",
        candidates[0].display(),
        candidates[1].display()
    )))
}

/// Transcribe an on-disk audio file. Backends are responsible for any
/// re-encoding the underlying service needs; callers pass the path to the
/// captured WAV.
pub trait Transcriber: Send + Sync {
    fn transcribe(&self, audio_path: &Path, language_hint: Option<&str>) -> Result<Transcript>;
}

#[cfg(test)]
mod write_read_tests {
    use super::*;
    use tempfile::TempDir;

    fn sample() -> SessionTranscript {
        SessionTranscript {
            channels: vec![ChannelTranscript {
                channel: "mic".into(),
                language: Some("en".into()),
                segments: vec![TranscriptSegment {
                    start_seconds: 0.0,
                    end_seconds: 1.0,
                    text: "Hello, world.".into(),
                    speaker: None,
                }],
            }],
        }
    }

    #[test]
    fn write_then_read_compressed_round_trips() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("transcript.json");
        let t = sample();
        t.write_json(&path).unwrap();
        // Writer creates .zst file, removes the plain one.
        let zst = path.with_extension("json.zst");
        assert!(zst.exists());
        assert!(!path.exists());
        // Reader resolves the .zst when given the canonical path.
        let read = SessionTranscript::read_json(&path).unwrap();
        assert_eq!(read.channels.len(), 1);
        assert_eq!(read.channels[0].segments[0].text, "Hello, world.");
    }

    #[test]
    fn read_back_compat_uncompressed_transcript() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("transcript.json");
        // Simulate a v1.0 transcript: plain JSON written by an older Attune.
        let json = serde_json::to_string_pretty(&sample()).unwrap();
        std::fs::write(&path, json).unwrap();
        let read = SessionTranscript::read_json(&path).unwrap();
        assert_eq!(read.channels[0].segments[0].text, "Hello, world.");
    }

    #[test]
    fn compressed_is_smaller_than_uncompressed_for_realistic_input() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("transcript.json");
        // Repetitive segments compress well — typical of real
        // meetings with lots of common phrases.
        let big = SessionTranscript {
            channels: vec![ChannelTranscript {
                channel: "mic".into(),
                language: Some("en".into()),
                segments: (0..200)
                    .map(|i| TranscriptSegment {
                        start_seconds: i as f64,
                        end_seconds: (i + 1) as f64,
                        text: format!("This is a fairly typical meeting sentence number {i}."),
                        speaker: None,
                    })
                    .collect(),
            }],
        };
        let raw = serde_json::to_vec_pretty(&big).unwrap();
        big.write_json(&path).unwrap();
        let zst_size = std::fs::metadata(path.with_extension("json.zst"))
            .unwrap()
            .len() as usize;
        assert!(
            zst_size < raw.len() / 2,
            "expected ≥2x shrink, got raw={} compressed={}",
            raw.len(),
            zst_size
        );
    }

    #[test]
    fn labeled_dialogue_merges_chronologically_and_numbers_speakers() {
        // Mic ("You") and a 2-speaker system channel, deliberately out of
        // time order across channels. Raw cluster indices are 4 and 2 —
        // they must surface as "Speaker 1"/"Speaker 2" by first appearance,
        // and every line must land in start-time order.
        let t = SessionTranscript {
            channels: vec![
                ChannelTranscript {
                    channel: "mic".into(),
                    language: None,
                    segments: vec![
                        TranscriptSegment {
                            start_seconds: 1.0,
                            end_seconds: 2.0,
                            text: "Kicking us off.".into(),
                            speaker: None,
                        },
                        TranscriptSegment {
                            start_seconds: 9.0,
                            end_seconds: 10.0,
                            text: "Thanks both.".into(),
                            speaker: None,
                        },
                    ],
                },
                ChannelTranscript {
                    channel: "system".into(),
                    language: None,
                    segments: vec![
                        TranscriptSegment {
                            start_seconds: 3.0,
                            end_seconds: 4.0,
                            text: "I'll take the design.".into(),
                            speaker: Some(4),
                        },
                        TranscriptSegment {
                            start_seconds: 6.0,
                            end_seconds: 7.0,
                            text: "I'll handle the backend.".into(),
                            speaker: Some(2),
                        },
                    ],
                },
            ],
        };

        let plain = t.to_labeled_dialogue(false);
        assert_eq!(
            plain,
            "You: Kicking us off.\n\
             Speaker 1: I'll take the design.\n\
             Speaker 2: I'll handle the backend.\n\
             You: Thanks both."
        );

        let stamped = t.to_labeled_dialogue(true);
        assert_eq!(
            stamped,
            "[0:01] You: Kicking us off.\n\
             [0:03] Speaker 1: I'll take the design.\n\
             [0:06] Speaker 2: I'll handle the backend.\n\
             [0:09] You: Thanks both."
        );
    }

    #[test]
    fn labeled_dialogue_falls_back_to_others_for_undiarized_system() {
        let t = SessionTranscript {
            channels: vec![ChannelTranscript {
                channel: "system".into(),
                language: None,
                segments: vec![TranscriptSegment {
                    start_seconds: 0.0,
                    end_seconds: 1.0,
                    text: "Some system audio.".into(),
                    speaker: None,
                }],
            }],
        };
        assert_eq!(t.to_labeled_dialogue(false), "Others: Some system audio.");
    }
}
