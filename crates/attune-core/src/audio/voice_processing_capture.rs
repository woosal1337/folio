//! Apple Voice Processing IO capture path (macOS only).
//!
//! Wraps `kAudioUnitSubType_VoiceProcessingIO` — the same AudioUnit
//! that ships inside Zoom, Meet, FaceTime, and Discord on macOS — so
//! the user can record without headphones and the mic stops capturing
//! whatever the laptop speakers are playing. VPIO bundles acoustic
//! echo cancellation, noise suppression, and automatic gain control
//! into one OS-managed processing pass.
//!
//! ## Module layout
//!
//! - [`ducking`] — private. Mirror of Apple's
//!   `AUVoiceIOOtherAudioDuckingConfiguration` struct + the helper
//!   that sets it to Min so macOS doesn't slam other audio output to
//!   ~5 % the moment we start recording.
//! - [`buffered`] — public. [`VoiceProcessingCapture`], an in-memory
//!   capture used by the `attune-cli vpio-smoke` standalone test.
//! - [`streaming`] — public. [`VoiceProcessingMicCapture`], the
//!   production type used by [`crate::audio::CaptureSession`].
//!
//! ## Why the reference signal is not plumbed
//!
//! Apple's VPIO operates at the OS level, not at the application
//! level. The AEC compares the mic input against the system's
//! outgoing speaker mix as seen by the OS, not against any reference
//! audio we pass in. That means we do NOT need to feed our
//! [`crate::audio::system`] (ScreenCaptureKit) capture as a
//! reference — the OS already sees the same audio at a lower layer.
//! This is the same reason Zoom does not "share" any audio with VPIO;
//! VPIO just works when the unit is active and the OS is emitting
//! sound to a real output device.

mod buffered;
mod ducking;
mod streaming;

pub use buffered::VoiceProcessingCapture;
pub use streaming::VoiceProcessingMicCapture;

/// Capture rate forced on VPIO. We use 16 kHz — VPIO is explicitly
/// tuned for voice bandwidth at this rate, it matches Whisper's
/// native input, and it's the rate that initialises cleanest across
/// the hardware matrix (M-series Macs, USB mics, AirPods). Higher
/// rates also work but unnecessarily inflate the captured WAV with
/// content above the voice band that Whisper would drop anyway.
pub const VPIO_SAMPLE_RATE_HZ: f64 = 16_000.0;

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: build and tear down a VPIO unit without ever
    /// starting it. Verifies the property-setting path works on this
    /// machine. Marked `#[ignore]` because it talks to the real
    /// CoreAudio subsystem and won't run in headless CI.
    #[test]
    #[ignore]
    fn instantiate_and_drop() {
        let cap = VoiceProcessingCapture::new().expect("VPIO new failed");
        assert_eq!(cap.sample_rate(), VPIO_SAMPLE_RATE_HZ);
    }
}
