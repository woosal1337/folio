//! Apple's `AUVoiceIOOtherAudioDuckingConfiguration` plumbing.
//!
//! Without this property set, macOS aggressively ducks every other
//! audio stream (Music, Safari, ScreenCaptureKit-rendered meeting
//! audio, etc.) down to ~5% volume the moment VPIO goes live — same
//! hard-coded behaviour that drops your system volume during a Zoom
//! call. The property was added in macOS 14 (Sonoma); on older
//! versions the `set_property` call is rejected and we fall back to
//! the system's default aggressive ducking.

use coreaudio::audio_unit::{AudioUnit, Element, Scope};
use coreaudio_sys::{
    kAUVoiceIOOtherAudioDuckingLevelMin, kAUVoiceIOProperty_OtherAudioDuckingConfiguration,
};
use tracing::{info, warn};

/// Mirror of Apple's `AUVoiceIOOtherAudioDuckingConfiguration` struct
/// from `<AudioToolbox/AUComponent.h>`, in C layout.
///
/// `Boolean` on Apple platforms is a 1-byte type; `#[repr(C)]` inserts
/// the 3 bytes of padding before the u32 automatically so the total
/// size matches the 8-byte C struct CoreAudio expects. We declare the
/// struct ourselves (not from coreaudio_sys) so the layout is
/// self-evident at the call site; the constants for the property ID
/// and ducking level come from coreaudio_sys to stay in sync with
/// whatever the bound macOS SDK headers say.
#[repr(C)]
struct OtherAudioDuckingConfiguration {
    enable_advanced_ducking: u8,
    ducking_level: u32,
}

/// Apply the minimum-ducking configuration to a VPIO unit. Call after
/// `initialize()` but before `start()`. Failure is logged at `warn`
/// level (older macOS rejects the property) but does not fail the
/// recording — the user just gets the default aggressive ducking.
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
