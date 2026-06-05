use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

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

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "observe" => Some(MemoryKind::Observe),
            "claim" => Some(MemoryKind::Claim),
            "pref" => Some(MemoryKind::Pref),
            "person" => Some(MemoryKind::Person),
            _ => None,
        }
    }

    pub fn is_keyed(&self) -> bool {
        !matches!(self, MemoryKind::Observe)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct Memory {
    pub id: String,
    pub kind: MemoryKind,

    pub key: Option<String>,

    pub content: String,

    pub evidence: Option<String>,

    pub confidence: f32,

    pub tags: Vec<String>,

    pub source_session_dir: Option<String>,

    pub source_session_label: Option<String>,

    pub valid_from: DateTime<Utc>,

    pub valid_until: Option<DateTime<Utc>>,

    pub supersedes_id: Option<String>,

    pub pinned: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    #[serde(skip)]
    #[ts(skip)]
    pub extras: BTreeMap<String, serde_norway::Value>,
}

impl Memory {
    pub fn is_current(&self) -> bool {
        self.valid_until.is_none()
    }
}

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

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct MemoryUpdate {
    pub content: Option<String>,
    pub key: Option<String>,
    pub evidence: Option<String>,
    pub tags: Option<Vec<String>>,
    pub pinned: Option<bool>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct MemoryQuery {
    pub query: Option<String>,

    #[serde(default)]
    pub kinds: Vec<MemoryKind>,

    #[serde(default)]
    pub include_archived: bool,

    pub limit: Option<usize>,
}
