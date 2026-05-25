//! Public types crossing the IPC boundary into the React frontend.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The kinds of memory a single-user meetings app needs. This is a
/// deliberate subset of Avid Brain's 10 event kinds (Observe / Claim /
/// Lesson / Pref / SkillEdit / Verify / Archive / Redact / Import /
/// Audit) — we don't need Lesson or SkillEdit yet, and Audit is
/// implicit in the supersede chain.
///
/// - `Observe` — free-form context the model thought worth keeping
///   ("user prefers async over sync standups").
/// - `Claim` — schema-typed fact addressable by `key`
///   (`user.company` = "Attune"). Supersedes prior claims with the
///   same key.
/// - `Pref` — user preference, also keyed and superseded
///   (`ui.theme` = "dark").
/// - `Person` — someone the user mentions; `key` is the canonicalised
///   handle ("ege", "alice-engineering"); `content` is the role + any
///   other notes.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, TS, PartialEq, Eq, Hash, Default)]
#[ts(export, export_to = "../../../src/shared/types/")]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    #[default]
    Observe,
    Claim,
    Pref,
    Person,
}

impl MemoryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryKind::Observe => "observe",
            MemoryKind::Claim => "claim",
            MemoryKind::Pref => "pref",
            MemoryKind::Person => "person",
        }
    }

    /// Parse a snake-case kind name. Named `parse` (not `from_str`)
    /// to avoid clashing with the standard `FromStr` trait method.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "observe" => Some(MemoryKind::Observe),
            "claim" => Some(MemoryKind::Claim),
            "pref" => Some(MemoryKind::Pref),
            "person" => Some(MemoryKind::Person),
            _ => None,
        }
    }

    /// True for kinds where a `key` is required (and conflict
    /// resolution applies). `Observe` is the only key-less kind.
    pub fn is_keyed(&self) -> bool {
        !matches!(self, MemoryKind::Observe)
    }
}

/// A single memory page on disk + its row in the FTS5 / vec indexes.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct Memory {
    /// UUIDv7 — time-sortable so a default sort by id is also
    /// chronological. Matches Avid Brain's `event_id` choice.
    pub id: String,
    pub kind: MemoryKind,
    /// Dotted handle ("user.company", "ui.theme"). None only when
    /// `kind == Observe`.
    pub key: Option<String>,
    /// The memory in one sentence. This is what gets injected into
    /// system prompts + ranked by FTS5.
    pub content: String,
    /// Quoted snippet from the source transcript supporting the
    /// claim. None for user-created memories.
    pub evidence: Option<String>,
    /// Model self-reported confidence (0-1). User-created memories
    /// default to 1.0.
    pub confidence: f32,
    /// Free-form tags, indexed at FTS5 weight 5x. ("identity",
    /// "engineering", "company", ...)
    pub tags: Vec<String>,
    /// Recording session this memory was extracted from. None for
    /// user-created memories.
    pub source_session_dir: Option<String>,
    /// Trailing path component of `source_session_dir`, surfaced as
    /// a deep link in the UI without re-deriving it.
    pub source_session_label: Option<String>,
    /// Inclusive lower bound on the validity window. Equal to
    /// `created_at` on first write.
    pub valid_from: DateTime<Utc>,
    /// When the memory stopped being currently-true. `None` means
    /// "still true". Set on supersede / user delete so we never hard
    /// delete history (Zep bi-temporal pattern).
    pub valid_until: Option<DateTime<Utc>>,
    /// Id of the prior memory this one replaces. `None` for the
    /// first memory of its kind+key.
    pub supersedes_id: Option<String>,
    /// True when the user has explicitly pinned this memory — pinned
    /// memories override the automatic "always-inject" set.
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Memory {
    /// Convenience: a currently-valid memory is one with no
    /// `valid_until` (or one set in the future, though we never do
    /// that today).
    pub fn is_current(&self) -> bool {
        self.valid_until.is_none()
    }
}

/// Constructor payload for [`crate::memory::MemoryStore::create`].
/// Mirrors `NewTask` — the store generates id, timestamps, and
/// validity bounds.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct NewMemory {
    pub kind: MemoryKind,
    pub key: Option<String>,
    pub content: String,
    pub evidence: Option<String>,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    #[serde(default)]
    pub tags: Vec<String>,
    pub source_session_dir: Option<String>,
    pub source_session_label: Option<String>,
}

fn default_confidence() -> f32 {
    1.0
}

/// Partial update used by `update_memory`. None fields are unchanged;
/// empty strings clear nullable scalars (matches `TaskUpdate`).
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct MemoryUpdate {
    pub content: Option<String>,
    pub key: Option<String>,
    pub evidence: Option<String>,
    pub tags: Option<Vec<String>>,
    pub pinned: Option<bool>,
}

/// Search/filter parameters. All fields are optional; the default
/// returns every currently-valid memory.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct MemoryQuery {
    /// Free-text query string. Empty → no text filter (returns by
    /// recency).
    pub query: Option<String>,
    /// Restrict to these kinds. Empty → all kinds.
    #[serde(default)]
    pub kinds: Vec<MemoryKind>,
    /// When true, include superseded / soft-deleted memories.
    /// Defaults to false (currently-valid only).
    #[serde(default)]
    pub include_archived: bool,
    /// Max rows returned.
    pub limit: Option<usize>,
}
