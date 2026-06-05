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

    pub fn disk_mb(self) -> u64 {
        match self {
            LocalLlmModel::Qwen25_7BInstruct => 4_700,
            LocalLlmModel::Llama31_8BInstruct => 4_900,
            LocalLlmModel::Phi35Mini => 2_400,
            LocalLlmModel::Gemma2_2BIt => 1_700,
        }
    }

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

pub fn recommend_default(ram_gb: u64) -> LocalLlmModel {
    if ram_gb >= MIN_RAM_GB_FOR_8B {
        LocalLlmModel::Qwen25_7BInstruct
    } else {
        LocalLlmModel::Phi35Mini
    }
}

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
        assert!(
            LocalLlmModel::Qwen25_7BInstruct.disk_mb()
                <= LocalLlmModel::Llama31_8BInstruct.disk_mb()
        );
    }
}
