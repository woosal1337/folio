//! On-disk markdown format for a memory page.
//!
//! Each memory is one markdown file with YAML frontmatter at the top
//! and a "compiled truth + timeline" body (the pattern GBrain uses for
//! its wiki pages). The frontmatter is the machine-readable contract;
//! the body is human-readable and git-diffable.
//!
//! We hand-roll a tiny YAML serializer rather than pulling in
//! `serde_yaml`. The frontmatter shape is fixed and tiny (~10 scalar
//! fields + two string lists), and we want byte-identical output
//! across writes so git diffs stay clean. `serde_yaml`'s key
//! ordering is not stable across versions.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::error::{AttuneError, Result};
use crate::memory::types::{Memory, MemoryKind};

/// Trailing path component of `dir` for a given memory id. Filename
/// format is `<kind>_<short-uuid>.md` — we don't include the key in
/// the filename because keys can be renamed and we want a stable file
/// path across renames. Short uuid is the leading 8 chars of the
/// UUIDv7 (sortable, almost always unique inside a single user's
/// store).
pub fn filename_for(memory: &Memory) -> String {
    let short = memory.id.split('-').next().unwrap_or(&memory.id);
    format!("{}_{}.md", memory.kind.as_str(), short)
}

/// Absolute path on disk for a memory.
pub fn path_for(dir: &Path, memory: &Memory) -> PathBuf {
    dir.join(filename_for(memory))
}

/// Write a memory page to disk. Atomic: writes to a sibling temp file
/// and renames into place so a crash mid-write cannot corrupt the
/// final file. Creates the parent directory tree on first write.
pub fn write_page(dir: &Path, memory: &Memory) -> Result<PathBuf> {
    fs::create_dir_all(dir).map_err(|e| {
        AttuneError::Storage(format!(
            "could not create memory dir {}: {e}",
            dir.display()
        ))
    })?;
    let path = path_for(dir, memory);
    let body = render_page(memory);
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, body).map_err(|e| {
        AttuneError::Storage(format!("could not write memory tmp {}: {e}", tmp.display()))
    })?;
    fs::rename(&tmp, &path).map_err(|e| {
        AttuneError::Storage(format!(
            "could not finalize memory file {}: {e}",
            path.display()
        ))
    })?;
    Ok(path)
}

/// Delete a memory file. Idempotent — missing file is success.
pub fn delete_page(dir: &Path, memory: &Memory) -> Result<()> {
    let path = path_for(dir, memory);
    match fs::remove_file(&path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AttuneError::Storage(format!(
            "could not remove memory file {}: {e}",
            path.display()
        ))),
    }
}

/// Read every memory page in `dir`. Files we can't parse are logged
/// and skipped rather than failing the load — a hand-edited file
/// shouldn't bring down the whole index rebuild.
pub fn read_dir_pages(dir: &Path) -> Vec<Memory> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        match fs::read_to_string(&path).and_then(|raw| {
            parse_page(&raw).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }) {
            Ok(memory) => out.push(memory),
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "skipping unreadable memory page",
                );
            }
        }
    }
    out
}

/// Render a memory as its markdown page (frontmatter + body).
pub fn render_page(memory: &Memory) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    push_str(&mut out, "id", &memory.id);
    push_str(&mut out, "kind", memory.kind.as_str());
    push_opt_str(&mut out, "key", memory.key.as_deref());
    push_opt_str(&mut out, "evidence", memory.evidence.as_deref());
    push_float(&mut out, "confidence", memory.confidence);
    push_string_list(&mut out, "tags", &memory.tags);
    push_opt_str(
        &mut out,
        "source_session_dir",
        memory.source_session_dir.as_deref(),
    );
    push_opt_str(
        &mut out,
        "source_session_label",
        memory.source_session_label.as_deref(),
    );
    push_str(&mut out, "valid_from", &memory.valid_from.to_rfc3339());
    push_opt_str(
        &mut out,
        "valid_until",
        memory
            .valid_until
            .as_ref()
            .map(|t| t.to_rfc3339())
            .as_deref(),
    );
    push_opt_str(&mut out, "supersedes_id", memory.supersedes_id.as_deref());
    push_bool(&mut out, "pinned", memory.pinned);
    push_str(&mut out, "created_at", &memory.created_at.to_rfc3339());
    push_str(&mut out, "updated_at", &memory.updated_at.to_rfc3339());
    out.push_str("---\n\n");

    // Body: heading is the key (or "Observation" for keyless),
    // followed by the current value and a placeholder timeline that
    // the renderer leaves for the user / future passes to expand.
    let heading = memory.key.as_deref().unwrap_or("Observation");
    out.push_str(&format!("# {heading}\n\n"));
    out.push_str(&format!("**Current:** {}\n", memory.content.trim()));
    if let Some(ev) = &memory.evidence {
        out.push_str(&format!("\n> {}\n", ev.trim()));
    }
    out
}

/// Parse a memory page from raw bytes. Returns an error string the
/// caller can log if the frontmatter is malformed.
pub fn parse_page(raw: &str) -> std::result::Result<Memory, String> {
    let rest = raw
        .strip_prefix("---\n")
        .ok_or_else(|| "missing leading frontmatter delimiter".to_string())?;
    let end = rest
        .find("\n---")
        .ok_or_else(|| "missing trailing frontmatter delimiter".to_string())?;
    let frontmatter = &rest[..end];

    let mut id: Option<String> = None;
    let mut kind: Option<MemoryKind> = None;
    let mut key: Option<String> = None;
    let mut content_line: Option<String> = None;
    let mut evidence: Option<String> = None;
    let mut confidence: f32 = 1.0;
    let mut tags: Vec<String> = Vec::new();
    let mut source_session_dir: Option<String> = None;
    let mut source_session_label: Option<String> = None;
    let mut valid_from: Option<DateTime<Utc>> = None;
    let mut valid_until: Option<DateTime<Utc>> = None;
    let mut supersedes_id: Option<String> = None;
    let mut pinned: bool = false;
    let mut created_at: Option<DateTime<Utc>> = None;
    let mut updated_at: Option<DateTime<Utc>> = None;

    for line in frontmatter.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (k, v) = line
            .split_once(':')
            .ok_or_else(|| format!("frontmatter line missing colon: {line}"))?;
        let k = k.trim();
        let v = v.trim();
        match k {
            "id" => id = Some(unquote(v)),
            "kind" => {
                let kind_str = unquote(v);
                kind = MemoryKind::parse(&kind_str)
                    .ok_or_else(|| format!("unknown kind: {v}"))
                    .map(Some)?
            }
            "key" => key = parse_opt_string(v),
            "evidence" => evidence = parse_opt_string(v),
            "confidence" => {
                confidence = v
                    .parse::<f32>()
                    .map_err(|e| format!("bad confidence: {e}"))?
            }
            "tags" => tags = parse_string_list(v),
            "source_session_dir" => source_session_dir = parse_opt_string(v),
            "source_session_label" => source_session_label = parse_opt_string(v),
            "valid_from" => {
                let s = unquote(v);
                valid_from = Some(
                    DateTime::parse_from_rfc3339(&s)
                        .map_err(|e| format!("bad valid_from: {e}"))?
                        .with_timezone(&Utc),
                )
            }
            "valid_until" => {
                valid_until = parse_opt_string(v)
                    .map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .map(|d| d.with_timezone(&Utc))
                            .map_err(|e| format!("bad valid_until: {e}"))
                    })
                    .transpose()?
            }
            "supersedes_id" => supersedes_id = parse_opt_string(v),
            "pinned" => pinned = v == "true",
            "created_at" => {
                let s = unquote(v);
                created_at = Some(
                    DateTime::parse_from_rfc3339(&s)
                        .map_err(|e| format!("bad created_at: {e}"))?
                        .with_timezone(&Utc),
                )
            }
            "updated_at" => {
                let s = unquote(v);
                updated_at = Some(
                    DateTime::parse_from_rfc3339(&s)
                        .map_err(|e| format!("bad updated_at: {e}"))?
                        .with_timezone(&Utc),
                )
            }
            _ => {
                // Unknown keys are ignored so future fields stay
                // backwards-compatible with older files.
            }
        }
    }

    // Body is after the trailing `---`. We extract the line that
    // begins with `**Current:**` as the canonical content, which keeps
    // hand-edited bodies sane (users can write any prose after it).
    let body = rest[end..].trim_start_matches("\n---").trim_start();
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("**Current:**") {
            content_line = Some(rest.trim().to_string());
            break;
        }
    }

    Ok(Memory {
        id: id.ok_or_else(|| "missing id".to_string())?,
        kind: kind.ok_or_else(|| "missing kind".to_string())?,
        key,
        content: content_line.unwrap_or_default(),
        evidence,
        confidence,
        tags,
        source_session_dir,
        source_session_label,
        valid_from: valid_from.ok_or_else(|| "missing valid_from".to_string())?,
        valid_until,
        supersedes_id,
        pinned,
        created_at: created_at.ok_or_else(|| "missing created_at".to_string())?,
        updated_at: updated_at.ok_or_else(|| "missing updated_at".to_string())?,
    })
}

// ---- tiny YAML emit/parse helpers --------------------------------

fn push_str(out: &mut String, k: &str, v: &str) {
    out.push_str(&format!("{}: {}\n", k, quote_if_needed(v)));
}

fn push_opt_str(out: &mut String, k: &str, v: Option<&str>) {
    match v {
        Some(s) => push_str(out, k, s),
        None => out.push_str(&format!("{}: null\n", k)),
    }
}

fn push_float(out: &mut String, k: &str, v: f32) {
    out.push_str(&format!("{}: {:.3}\n", k, v));
}

fn push_bool(out: &mut String, k: &str, v: bool) {
    out.push_str(&format!("{}: {}\n", k, v));
}

fn push_string_list(out: &mut String, k: &str, items: &[String]) {
    if items.is_empty() {
        out.push_str(&format!("{}: []\n", k));
        return;
    }
    let inner: Vec<String> = items.iter().map(|s| quote_if_needed(s)).collect();
    out.push_str(&format!("{}: [{}]\n", k, inner.join(", ")));
}

/// Quote strings that contain YAML-special characters. Everything
/// else round-trips bare for readable frontmatter.
fn quote_if_needed(s: &str) -> String {
    let needs_quote = s.is_empty()
        || s.chars().any(|c| {
            matches!(
                c,
                ':' | '#' | '\'' | '"' | '\n' | ',' | '[' | ']' | '{' | '}'
            )
        });
    if needs_quote {
        // Always-quote uses double quotes + \"-escape for safety.
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

/// Strip surrounding quotes and undo the `\\` + `\"` escaping
/// `quote_if_needed` applies. Returns owned String because the
/// unescape path needs to mutate.
fn unquote(v: &str) -> String {
    let v = v.trim();
    let is_quoted = v.len() >= 2
        && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')));
    if !is_quoted {
        return v.to_string();
    }
    let inner = &v[1..v.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_opt_string(v: &str) -> Option<String> {
    let v = v.trim();
    if v == "null" || v.is_empty() {
        None
    } else {
        Some(unquote(v))
    }
}

fn parse_string_list(v: &str) -> Vec<String> {
    let v = v.trim();
    if v == "[]" || v.is_empty() {
        return Vec::new();
    }
    let inner = v.trim_start_matches('[').trim_end_matches(']');
    inner
        .split(',')
        .map(|s| unquote(s.trim()))
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::MemoryKind;
    use chrono::TimeZone;

    fn sample() -> Memory {
        Memory {
            id: "01H9X5ABCDEFGHIJKLMNOPQRST".to_string(),
            kind: MemoryKind::Claim,
            key: Some("user.company".to_string()),
            content: "Attune.".to_string(),
            evidence: Some("\"I work at Attune now\"".to_string()),
            confidence: 0.92,
            tags: vec!["company".to_string(), "identity".to_string()],
            source_session_dir: Some("/Recordings/2026-05-25".to_string()),
            source_session_label: Some("2026-05-25".to_string()),
            valid_from: Utc.with_ymd_and_hms(2026, 5, 25, 14, 0, 0).unwrap(),
            valid_until: None,
            supersedes_id: Some("01H9W4...".to_string()),
            pinned: false,
            created_at: Utc.with_ymd_and_hms(2026, 5, 25, 14, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2026, 5, 25, 14, 0, 0).unwrap(),
        }
    }

    #[test]
    fn round_trip_preserves_every_field() {
        let m = sample();
        let raw = render_page(&m);
        let parsed = parse_page(&raw).expect("parse");
        assert_eq!(parsed.id, m.id);
        assert_eq!(parsed.kind, m.kind);
        assert_eq!(parsed.key, m.key);
        assert_eq!(parsed.content, m.content);
        assert_eq!(parsed.evidence, m.evidence);
        assert!((parsed.confidence - m.confidence).abs() < 0.01);
        assert_eq!(parsed.tags, m.tags);
        assert_eq!(parsed.source_session_label, m.source_session_label);
        assert_eq!(parsed.valid_from, m.valid_from);
        assert_eq!(parsed.valid_until, m.valid_until);
        assert_eq!(parsed.supersedes_id, m.supersedes_id);
        assert_eq!(parsed.pinned, m.pinned);
        assert_eq!(parsed.created_at, m.created_at);
        assert_eq!(parsed.updated_at, m.updated_at);
    }

    #[test]
    fn quotes_yaml_special_chars() {
        let mut m = sample();
        m.content = "a: with colons, [and brackets]".into();
        let raw = render_page(&m);
        let parsed = parse_page(&raw).expect("parse");
        assert_eq!(parsed.content, m.content);
    }

    #[test]
    fn observation_renders_with_observation_heading() {
        let mut m = sample();
        m.kind = MemoryKind::Observe;
        m.key = None;
        let raw = render_page(&m);
        assert!(raw.contains("# Observation"));
    }

    #[test]
    fn parse_handles_null_optionals() {
        let mut m = sample();
        m.key = None;
        m.evidence = None;
        m.source_session_dir = None;
        m.source_session_label = None;
        m.supersedes_id = None;
        m.tags = Vec::new();
        m.valid_until = None;
        m.kind = MemoryKind::Observe;
        let raw = render_page(&m);
        let parsed = parse_page(&raw).expect("parse");
        assert!(parsed.key.is_none());
        assert!(parsed.evidence.is_none());
        assert!(parsed.source_session_dir.is_none());
        assert!(parsed.supersedes_id.is_none());
        assert!(parsed.tags.is_empty());
        assert!(parsed.valid_until.is_none());
    }

    #[test]
    fn read_dir_pages_skips_unparsable_files() {
        let dir = tempfile::tempdir().unwrap();
        write_page(dir.path(), &sample()).unwrap();
        fs::write(dir.path().join("garbage.md"), "not a memory").unwrap();
        fs::write(dir.path().join("notes.txt"), "ignored").unwrap();
        let memories = read_dir_pages(dir.path());
        assert_eq!(memories.len(), 1);
    }

    #[test]
    fn delete_page_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let m = sample();
        write_page(dir.path(), &m).unwrap();
        delete_page(dir.path(), &m).unwrap();
        delete_page(dir.path(), &m).unwrap();
        assert!(read_dir_pages(dir.path()).is_empty());
    }
}
