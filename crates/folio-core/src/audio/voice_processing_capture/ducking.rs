use coreaudio::audio_unit::{AudioUnit, Element, Scope};
use coreaudio_sys::{
    kAUVoiceIOOtherAudioDuckingLevelMin, kAUVoiceIOProperty_OtherAudioDuckingConfiguration,
};
use tracing::{info, warn};

#[repr(C)]
struct OtherAudioDuckingConfiguration {
    enable_advanced_ducking: u8,
    ducking_level: u32,
}

pub(super) fn apply_minimum_ducking(audio_unit: &mut AudioUnit) {
    let cfg = OtherAudioDuckingConfiguration {
        enable_advanced_ducking: 0,
        ducking_level: kAUVoiceIOOtherAudioDuckingLevelMin,
    };
    match audio_unit.set_property(
        kAUVoiceIOProperty_OtherAudioDuckingConfiguration,
        Scope::Global,
        Element::Output,
        Some(&cfg),
    ) {
        Ok(()) => info!(
            level = kAUVoiceIOOtherAudioDuckingLevelMin,
            "VPIO other-audio ducking set to Min",
        ),
        Err(e) => warn!(
            error = %e,
            "VPIO ducking config rejected — system default ducking will apply",
        ),
    }
}
