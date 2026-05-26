//! Local LLM catalogue + selection. v2 finding 049 / GET-43.
//!
//! Bundles a `llama-cpp-rs` runtime with a model picker that mirrors
//! the Local Whisper UX (#040). Defaults to a quantized 7B/8B with
//! native tool calling so the user can opt fully offline (#048
//! Privacy Mode) and still get task / memory extraction.
//!
//! This module owns the catalogue, the recommendation policy, and
//! the model-file path layout. The actual `llama-cpp-rs` runtime +
//! Metal kernel build live behind a `local_llm` cargo feature in the
//! follow-up (gated to keep the cold build time low for users who
//! never opt in).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MODEL_CACHE_SUBDIR: &str = "llama-models";
pub const DEFAULT_QUANTIZATION: &str = "Q4_K_M";
pub const MIN_RAM_GB_FOR_8B: u64 = 16;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
#[serde(rename_all = "snake_case")]
pub enum LocalLlmModel {
    Qwen25_7BInstruct,
    Llama31_8BInstruct,
    Phi35Mini,
    Gemma2_2BIt,
}

impl LocalLlmModel {
    pub fn id(self) -> &'static str {
        match self {
            LocalLlmModel::Qwen25_7BInstruct => "qwen2.5-7b-instruct",
            LocalLlmModel::Llama31_8BInstruct => "llama-3.1-8b-instruct",
            LocalLlmModel::Phi35Mini => "phi-3.5-mini-instruct",
            LocalLlmModel::Gemma2_2BIt => "gemma-2-2b-it",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            LocalLlmModel::Qwen25_7BInstruct => "Qwen 2.5 7B Instruct",
            LocalLlmModel::Llama31_8BInstruct => "Llama 3.1 8B Instruct",
            LocalLlmModel::Phi35Mini => "Phi 3.5 Mini Instruct",
            LocalLlmModel::Gemma2_2BIt => "Gemma 2 2B IT",
        }
    }

    /// Approximate quantised on-disk size in megabytes for the
    /// shipped Q4_K_M quantisation. Used by the disk-budget check
    /// before downloading.
    pub fn disk_mb(self) -> u64 {
        match self {
            LocalLlmModel::Qwen25_7BInstruct => 4_700,
            LocalLlmModel::Llama31_8BInstruct => 4_900,
            LocalLlmModel::Phi35Mini => 2_400,
            LocalLlmModel::Gemma2_2BIt => 1_700,
        }
    }

    /// True when the model has trained-in tool-calling that the
    /// agent runner can drive (`create_task`, `remember`, ...).
    pub fn supports_tool_calling(self) -> bool {
        matches!(
            self,
            LocalLlmModel::Qwen25_7BInstruct | LocalLlmModel::Llama31_8BInstruct
        )
    }

    pub fn from_id(id: &str) -> Option<LocalLlmModel> {
        Some(match id {
            "qwen2.5-7b-instruct" => LocalLlmModel::Qwen25_7BInstruct,
            "llama-3.1-8b-instruct" => LocalLlmModel::Llama31_8BInstruct,
            "phi-3.5-mini-instruct" => LocalLlmModel::Phi35Mini,
            "gemma-2-2b-it" => LocalLlmModel::Gemma2_2BIt,
            _ => return None,
        })
    }
}

/// Recommend a sensible default given the user's machine size. >= 16
/// GB RAM gets a 7B/8B with tool calling. Below that we fall back
/// to Phi-3.5 Mini, which still handles summarise / autoname well.
pub fn recommend_default(ram_gb: u64) -> LocalLlmModel {
    if ram_gb >= MIN_RAM_GB_FOR_8B {
        LocalLlmModel::Qwen25_7BInstruct
    } else {
        LocalLlmModel::Phi35Mini
    }
}

/// Catalogue the picker UI iterates over.
pub fn catalogue() -> &'static [LocalLlmModel] {
    &[
        LocalLlmModel::Qwen25_7BInstruct,
        LocalLlmModel::Llama31_8BInstruct,
        LocalLlmModel::Phi35Mini,
        LocalLlmModel::Gemma2_2BIt,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for model in catalogue() {
            assert_eq!(LocalLlmModel::from_id(model.id()), Some(*model));
        }
        assert!(LocalLlmModel::from_id("does-not-exist").is_none());
    }

    #[test]
    fn display_names_are_unique() {
        let mut names: Vec<&str> = catalogue().iter().map(|m| m.display_name()).collect();
        names.sort();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before);
    }

    #[test]
    fn recommend_default_picks_qwen_on_16gb_plus() {
        assert_eq!(recommend_default(16), LocalLlmModel::Qwen25_7BInstruct);
        assert_eq!(recommend_default(32), LocalLlmModel::Qwen25_7BInstruct);
    }

    #[test]
    fn recommend_default_falls_back_to_phi_on_smaller_machines() {
        assert_eq!(recommend_default(8), LocalLlmModel::Phi35Mini);
        assert_eq!(recommend_default(4), LocalLlmModel::Phi35Mini);
    }

    #[test]
    fn tool_calling_capability_is_7b_8b_only() {
        assert!(LocalLlmModel::Qwen25_7BInstruct.supports_tool_calling());
        assert!(LocalLlmModel::Llama31_8BInstruct.supports_tool_calling());
        assert!(!LocalLlmModel::Phi35Mini.supports_tool_calling());
        assert!(!LocalLlmModel::Gemma2_2BIt.supports_tool_calling());
    }

    #[test]
    fn disk_mb_is_monotonic_with_parameter_count() {
        assert!(LocalLlmModel::Gemma2_2BIt.disk_mb() < LocalLlmModel::Phi35Mini.disk_mb());
        assert!(LocalLlmModel::Phi35Mini.disk_mb() < LocalLlmModel::Qwen25_7BInstruct.disk_mb());
        assert!(LocalLlmModel::Qwen25_7BInstruct.disk_mb() <= LocalLlmModel::Llama31_8BInstruct.disk_mb());
    }
}
