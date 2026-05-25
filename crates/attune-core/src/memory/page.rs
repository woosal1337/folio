//! On-disk markdown format for a memory page.
//!
//! Each memory is one markdown file with YAML frontmatter at the top
//! and a "compiled truth + timeline" body (the pattern GBrain uses for
//! its wiki pages). The frontmatter is the machine-readable contract;
//! the body is human-readable and git-diffable.
//!
//! Parsing goes through `serde_norway` deserializing into the typed
//! `MemoryFrontmatter` struct below. Unknown keys are caught by a
//! `#[serde(flatten)]` extras map so a user who hand-edits a page and
//! adds their own field doesn't see it disappear on the next round-
//! trip — the writer emits the extras at the end of the frontmatter
//! block in sorted order. v2 finding 040 / GET-61.
//!
//! Rendering uses a small hand-rolled writer rather than
//! `serde_norway::to_string` so the byte output is stable across
//! versions and round-trips produce clean git diffs.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{AttuneError, Result};
use crate::memory::types::{Memory, MemoryKind};

/// Typed mirror of the on-disk frontmatter. Every known field is
/// reified as a strongly-typed property; everything else is caught by
/// the `#[serde(flatten)]` extras map so it can be re-emitted on write.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryFrontmatter {
    id: String,
    kind: String,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    evidence: Option<String>,
    #[serde(default = "default_confidence")]
    confidence: f32,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    source_session_dir: Option<String>,
    #[serde(default)]
    source_session_label: Option<String>,
    valid_from: String,
    #[serde(default)]
    valid_until: Option<String>,
    #[serde(default)]
    supersedes_id: Option<String>,
    #[serde(default)]
    pinned: bool,
    created_at: String,
    updated_at: String,
    /// Catch-all for user-added frontmatter keys. Preserved on
    /// round-trip.
    #[serde(flatten)]
    extras: BTreeMap<String, serde_norway::Value>,
}

fn default_confidence() -> f32 {
    1.0
}

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

/// Render a memory as its markdown page (frontmatter + body). The
/// frontmatter is emitted in a fixed order so git diffs stay clean;
/// any user-added unknown keys (carried through `Memory::extras`) are
/// appended at the end in BTree-sorted order.
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

    // User-added unknown keys, sorted for stable diffs. Emitted via
    // serde_norway so nested structures (lists, maps) survive a round-trip.
    for (k, v) in &memory.extras {
        // Skip any extras that happen to shadow a known field — the
        // known field takes precedence, and re-emitting the extra would
        // produce a duplicate key.
        if is_known_key(k) {
            continue;
        }
        match render_extra(k, v) {
            Ok(line) => out.push_str(&line),
            Err(e) => {
                tracing::warn!(key = %k, error = %e, "could not render memory extra; dropping");
            }
        }
    }

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
    let frontmatter_yaml = &rest[..end];

    let fm: MemoryFrontmatter = serde_norway::from_str(frontmatter_yaml)
        .map_err(|e| format!("frontmatter parse error: {e}"))?;

    let kind = MemoryKind::parse(&fm.kind).ok_or_else(|| format!("unknown kind: {}", fm.kind))?;

    let parse_dt = |label: &str, s: &str| -> std::result::Result<DateTime<Utc>, String> {
        DateTime::parse_from_rfc3339(s)
            .map(|d| d.with_timezone(&Utc))
            .map_err(|e| format!("bad {label}: {e}"))
    };

    let valid_from = parse_dt("valid_from", &fm.valid_from)?;
    let valid_until = fm
        .valid_until
        .as_deref()
        .map(|s| parse_dt("valid_until", s))
        .transpose()?;
    let created_at = parse_dt("created_at", &fm.created_at)?;
    let updated_at = parse_dt("updated_at", &fm.updated_at)?;

    // Body: pull the `**Current:**` line as the canonical content so
    // hand-edited prose around it is ignored.
    let body = rest[end..].trim_start_matches("\n---").trim_start();
    let mut content_line: Option<String> = None;
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("**Current:**") {
            content_line = Some(rest.trim().to_string());
            break;
        }
    }

    Ok(Memory {
        id: fm.id,
        kind,
        key: fm.key,
        content: content_line.unwrap_or_default(),
        evidence: fm.evidence,
        confidence: fm.confidence,
        tags: fm.tags,
        source_session_dir: fm.source_session_dir,
        source_session_label: fm.source_session_label,
        valid_from,
        valid_until,
        supersedes_id: fm.supersedes_id,
        pinned: fm.pinned,
        created_at,
        updated_at,
        extras: fm.extras,
    })
}

// ---- frontmatter render helpers ----------------------------------

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

/// Emit a single extra `key: value` line. Scalars round-trip through
/// `quote_if_needed`; non-scalars defer to `serde_norway`'s default
/// serializer with the YAML document markers stripped.
fn render_extra(key: &str, value: &serde_norway::Value) -> std::result::Result<String, String> {
    match value {
        serde_norway::Value::Null => Ok(format!("{}: null\n", key)),
        serde_norway::Value::Bool(b) => Ok(format!("{}: {}\n", key, b)),
        serde_norway::Value::Number(n) => Ok(format!("{}: {}\n", key, n)),
        serde_norway::Value::String(s) => Ok(format!("{}: {}\n", key, quote_if_needed(s))),
        other => {
            // Sequences and mappings — let serde_norway render them but
            // strip the document-start `---\n` marker so we don't get
            // nested document syntax.
            let rendered = serde_norway::to_string(&serde_norway::Value::Mapping({
                let mut m = serde_norway::Mapping::new();
                m.insert(serde_norway::Value::String(key.to_string()), other.clone());
                m
            }))
            .map_err(|e| e.to_string())?;
            Ok(rendered)
        }
    }
}

/// Known frontmatter keys — used to drop any catch-all that happens to
/// duplicate a typed field so the rendered page can't grow duplicates.
fn is_known_key(k: &str) -> bool {
    matches!(
        k,
        "id" | "kind"
            | "key"
            | "evidence"
            | "confidence"
            | "tags"
            | "source_session_dir"
            | "source_session_label"
            | "valid_from"
            | "valid_until"
            | "supersedes_id"
            | "pinned"
            | "created_at"
            | "updated_at"
    )
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
            extras: BTreeMap::new(),
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

    #[test]
    fn parse_preserves_unknown_extras_in_round_trip() {
        // Build a page on disk by hand, including an unknown user
        // field. The first parse-then-render cycle should put the
        // extra into Memory::extras and re-emit it; the second parse
        // should see the same extras.
        let raw = "---
id: 01H9XABCDEFGHIJKLMNOPQRST
kind: claim
key: user.company
evidence: null
confidence: 1.000
tags: []
source_session_dir: null
source_session_label: null
valid_from: 2026-05-25T14:00:00+00:00
valid_until: null
supersedes_id: null
pinned: false
created_at: 2026-05-25T14:00:00+00:00
updated_at: 2026-05-25T14:00:00+00:00
custom_field: hello
priority: 7
---

# user.company

**Current:** Attune.
";
        let parsed = parse_page(raw).expect("parse");
        assert_eq!(
            parsed.extras.get("custom_field").and_then(|v| v.as_str()),
            Some("hello"),
        );
        assert_eq!(
            parsed.extras.get("priority").and_then(|v| v.as_i64()),
            Some(7),
        );

        let rendered = render_page(&parsed);
        assert!(rendered.contains("custom_field: hello"));
        assert!(rendered.contains("priority: 7"));

        let reparsed = parse_page(&rendered).expect("reparse");
        assert_eq!(reparsed.extras, parsed.extras);
    }
}
