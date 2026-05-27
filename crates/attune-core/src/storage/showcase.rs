//! `.attune/showcase.md` — the Linktree-style portfolio file that the
//! eventual attune.app/u/<handle> page reads from. Local-first: the
//! source of truth lives in the user's vault, the public page is
//! just a renderer.
//!
//! Schema is intentionally tiny:
//!
//! ```yaml
//! ---
//! handle: ege
//! display_name: Ege Çelebi
//! bio: Builds things in public.
//! ---
//!
//! - title: Pricing sync with Lila
//!   url: https://attune.app/s/abc123
//! - title: Q3 planning offsite
//!   url: https://attune.app/s/def456
//! ```
//!
//! v2 roadmap finding 087 / GET-107. The web renderer is a separate
//! follow-up; this PR ships the on-disk format + the Settings UI
//! that helps the user start one.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AttuneError, Result};

const SHOWCASE_FILENAME: &str = "showcase.md";

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ShowcaseEntry {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct Showcase {
    pub handle: String,
    pub display_name: String,
    pub bio: String,
    pub entries: Vec<ShowcaseEntry>,
}

/// Path to the user's showcase file. Lives next to the
/// outbox / inbox under `.attune/`.
pub fn path_for(memory_dir: &Path) -> PathBuf {
    memory_dir.join(".attune").join(SHOWCASE_FILENAME)
}

/// Render a Showcase struct as the markdown shape documented above.
/// Stable bytes for git diff cleanliness.
pub fn render(showcase: &Showcase) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("handle: {}\n", showcase.handle));
    out.push_str(&format!("display_name: {}\n", showcase.display_name));
    out.push_str(&format!("bio: {}\n", showcase.bio.replace('\n', " ")));
    out.push_str("---\n\n");
    for e in &showcase.entries {
        out.push_str(&format!("- title: {}\n", e.title.replace('\n', " ")));
        out.push_str(&format!("  url: {}\n", e.url));
    }
    out
}

/// Parse the showcase file or return `Ok(None)` if it doesn't exist
/// yet (a fresh install). Returns an error only on read or parse
/// failure.
pub fn read(memory_dir: &Path) -> Result<Option<Showcase>> {
    let path = path_for(memory_dir);
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(AttuneError::Storage(format!(
                "could not read showcase {}: {e}",
                path.display()
            )));
        }
    };
    parse(&raw).map(Some)
}

/// Write the showcase file. Creates `.attune/` if missing.
pub fn write(memory_dir: &Path, showcase: &Showcase) -> Result<PathBuf> {
    let dir = memory_dir.join(".attune");
    fs::create_dir_all(&dir)
        .map_err(|e| AttuneError::Storage(format!("create_dir_all {}: {e}", dir.display())))?;
    let path = dir.join(SHOWCASE_FILENAME);
    let body = render(showcase);
    fs::write(&path, body)
        .map_err(|e| AttuneError::Storage(format!("write showcase {}: {e}", path.display())))?;
    Ok(path)
}

/// Parse the simple shape — frontmatter + entry list. Tolerant to
/// extra whitespace and missing keys (returns defaults rather than
/// erroring) so a hand-edited file doesn't lock the UI out.
fn parse(raw: &str) -> Result<Showcase> {
    let mut handle = String::new();
    let mut display_name = String::new();
    let mut bio = String::new();
    let mut entries: Vec<ShowcaseEntry> = Vec::new();

    let after_open = raw
        .strip_prefix("---\n")
        .or_else(|| raw.strip_prefix("---\r\n"))
        .unwrap_or(raw);
    if let Some(end) = after_open.find("\n---") {
        let frontmatter = &after_open[..end];
        for line in frontmatter.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(value) = trimmed.strip_prefix("handle:") {
                handle = value.trim().to_string();
            } else if let Some(value) = trimmed.strip_prefix("display_name:") {
                display_name = value.trim().to_string();
            } else if let Some(value) = trimmed.strip_prefix("bio:") {
                bio = value.trim().to_string();
            }
        }

        let body = &after_open[end..];
        let body = body.trim_start_matches("\n---").trim_start();
        let mut pending_title: Option<String> = None;
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("- title:") {
                if let Some(title) = pending_title.take() {
                    // Previous title without a url — surface as
                    // an entry pointing at the placeholder.
                    entries.push(ShowcaseEntry {
                        title,
                        url: String::new(),
                    });
                }
                pending_title = Some(rest.trim().to_string());
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("url:") {
                if let Some(title) = pending_title.take() {
                    entries.push(ShowcaseEntry {
                        title,
                        url: rest.trim().to_string(),
                    });
                }
            }
        }
        if let Some(title) = pending_title.take() {
            entries.push(ShowcaseEntry {
                title,
                url: String::new(),
            });
        }
    }

    Ok(Showcase {
        handle,
        display_name,
        bio,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Showcase {
        Showcase {
            handle: "ege".into(),
            display_name: "Ege Çelebi".into(),
            bio: "Builds things in public.".into(),
            entries: vec![
                ShowcaseEntry {
                    title: "Pricing sync with Lila".into(),
                    url: "https://attune.app/s/abc123".into(),
                },
                ShowcaseEntry {
                    title: "Q3 planning offsite".into(),
                    url: "https://attune.app/s/def456".into(),
                },
            ],
        }
    }

    #[test]
    fn round_trip_preserves_fields() {
        let s = sample();
        let raw = render(&s);
        let parsed = parse(&raw).unwrap();
        assert_eq!(parsed.handle, s.handle);
        assert_eq!(parsed.display_name, s.display_name);
        assert_eq!(parsed.bio, s.bio);
        assert_eq!(parsed.entries.len(), s.entries.len());
        assert_eq!(parsed.entries[0].title, s.entries[0].title);
    }

    #[test]
    fn read_returns_none_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(dir.path()).unwrap().is_none());
    }

    #[test]
    fn write_then_read_round_trips_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let written = write(dir.path(), &sample()).unwrap();
        assert!(written.exists());
        let loaded = read(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.handle, "ege");
        assert_eq!(loaded.entries.len(), 2);
    }
}
