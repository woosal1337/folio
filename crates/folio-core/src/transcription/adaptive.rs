use super::models::WhisperModel;

const LOW_BATTERY_PCT: u32 = 40;

pub fn recommend_default_model(memsize_gb: u64, metal: bool) -> WhisperModel {
    match (metal, memsize_gb) {
        (true, m) if m >= 32 => WhisperModel::LargeV3,
        (true, m) if m >= 16 => WhisperModel::Medium,
        (true, m) if m >= 8 => WhisperModel::Small,
        (true, _) => WhisperModel::Base,
        (false, m) if m >= 32 => WhisperModel::Medium,
        (false, m) if m >= 16 => WhisperModel::Small,
        (false, m) if m >= 8 => WhisperModel::Base,
        (false, _) => WhisperModel::Tiny,
    }
}

pub fn downgrade_for_power(
    configured: WhisperModel,
    on_battery: Option<bool>,
    battery_pct: Option<u32>,
) -> WhisperModel {
    let (Some(true), Some(pct)) = (on_battery, battery_pct) else {
        return configured;
    };
    if pct >= LOW_BATTERY_PCT {
        return configured;
    }
    match configured {
        WhisperModel::LargeV3 => WhisperModel::Medium,
        WhisperModel::Medium => WhisperModel::Small,
        WhisperModel::Small => WhisperModel::Base,
        WhisperModel::Base => WhisperModel::Tiny,
        WhisperModel::Tiny => WhisperModel::Tiny,
    }
}

pub fn detect_memsize_gb() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let out = Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()?;
        let bytes: u64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
        Some(bytes / (1024 * 1024 * 1024))
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

pub fn detect_metal() -> bool {
    cfg!(target_os = "macos")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommend_default_model_picks_large_for_apple_silicon_32gb() {
        assert_eq!(recommend_default_model(32, true), WhisperModel::LargeV3);
        assert_eq!(recommend_default_model(48, true), WhisperModel::LargeV3);
    }

    #[test]
    fn recommend_default_model_picks_medium_for_apple_silicon_16gb() {
        assert_eq!(recommend_default_model(16, true), WhisperModel::Medium);
    }

    #[test]
    fn recommend_default_model_steps_down_on_intel() {
        assert_eq!(recommend_default_model(32, false), WhisperModel::Medium);
        assert_eq!(recommend_default_model(16, false), WhisperModel::Small);
        assert_eq!(recommend_default_model(4, false), WhisperModel::Tiny);
    }

    #[test]
    fn downgrade_for_power_keeps_config_when_plugged_in() {
        let model = downgrade_for_power(WhisperModel::LargeV3, Some(false), Some(10));
        assert_eq!(model, WhisperModel::LargeV3);
    }

    #[test]
    fn downgrade_for_power_kicks_in_below_threshold() {
        assert_eq!(
            downgrade_for_power(WhisperModel::LargeV3, Some(true), Some(20)),
            WhisperModel::Medium
        );
        assert_eq!(
            downgrade_for_power(WhisperModel::Tiny, Some(true), Some(5)),
            WhisperModel::Tiny
        );
    }

    #[test]
    fn downgrade_for_power_keeps_config_when_battery_info_missing() {
        let model = downgrade_for_power(WhisperModel::LargeV3, None, None);
        assert_eq!(model, WhisperModel::LargeV3);
    }
}
