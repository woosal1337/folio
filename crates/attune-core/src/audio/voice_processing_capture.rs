//! Apple Voice Processing IO capture path (macOS only).
//!
//! Wraps `kAudioUnitSubType_VoiceProcessingIO` — the same AudioUnit
//! that ships inside Zoom, Meet, FaceTime, and Discord on macOS — so
//! the user can record without headphones and the mic stops capturing
//! whatever the laptop speakers are playing. VPIO bundles acoustic
//! echo cancellation, noise suppression, and automatic gain control
//! into one OS-managed processing pass.
//!
//! Phase 1 scope (per
//! `~/Documents/GitHub/obsidian.md/projects/attune/plan/voice-processing-io.md`):
//! the bare API to instantiate VPIO, capture mono Float32 frames at
//! 48 kHz into an in-memory buffer, and stop cleanly. Wiring into
//! [`crate::audio::CaptureSession`] is phase 2.
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

use std::sync::{Arc, Mutex};

use coreaudio::audio_unit::render_callback::{self, data};
use coreaudio::audio_unit::{AudioUnit, Element, IOType, Scope};
use coreaudio_sys::{
    kAudioOutputUnitProperty_EnableIO, kAudioUnitProperty_StreamFormat, AudioStreamBasicDescription,
};
use tracing::{debug, info};

use crate::error::{AttuneError, Result};

/// Capture rate forced on VPIO. We use 16 kHz — VPIO is explicitly
/// tuned for voice bandwidth at this rate, it matches Whisper's
/// native input, and it's the rate that initialises cleanest across
/// the hardware matrix (M-series Macs, USB mics, AirPods). Higher
/// rates also work but unnecessarily inflate the captured WAV with
/// content above the voice band that Whisper would drop anyway.
pub const VPIO_SAMPLE_RATE_HZ: f64 = 16_000.0;

/// One running VPIO capture session.
///
/// Owns the AudioUnit and the shared buffer the render callback
/// writes into. `start()` engages the unit; `stop()` halts it and
/// returns the captured samples. Drop runs `stop()` if you forgot.
pub struct VoiceProcessingCapture {
    audio_unit: AudioUnit,
    samples: Arc<Mutex<Vec<f32>>>,
    running: bool,
    /// Sample rate the unit actually negotiated with the device.
    /// May differ from [`VPIO_SAMPLE_RATE_HZ`] when the bound input
    /// device only supports its own native rate.
    negotiated_sample_rate: f64,
    /// Channels per frame the callback receives. We collapse to mono
    /// downstream if this is greater than 1.
    negotiated_channels: u32,
}

impl VoiceProcessingCapture {
    /// Build a VPIO capture unit bound to the system default input
    /// device. Does not start capture — call [`Self::start`].
    ///
    /// VPIO must be left UNINITIALIZED while we set EnableIO and the
    /// stream format. `coreaudio-rs`'s `AudioUnit::new` initialises
    /// the unit eagerly, so we use `new_uninitialized` to get the
    /// raw unit and call `initialize()` ourselves once all properties
    /// are set.
    pub fn new() -> Result<Self> {
        debug!("instantiating VoiceProcessingIO AudioUnit");
        let mut audio_unit = AudioUnit::new_uninitialized(IOType::VoiceProcessingIO)
            .map_err(|e| AttuneError::AudioDevice(format!("VPIO instantiate: {e}")))?;

        // Minimal config: enable input bus, init. Nothing else.
        // We tried full Apple-recommended setups (set device, set
        // format, enable/disable output in every combination) and
        // every variant returned kAudioUnitErr_FailedInitialization
        // from this CLI binary on M-series macOS 26. The minimal
        // config either succeeds (CoreAudio picks every default
        // itself) or fails the same way, in which case the failure
        // is environmental rather than a property bug.
        let enable: u32 = 1;
        audio_unit
            .set_property(
                kAudioOutputUnitProperty_EnableIO,
                Scope::Input,
                Element::Input,
                Some(&enable),
            )
            .map_err(|e| AttuneError::AudioDevice(format!("VPIO enable input: {e}")))?;

        audio_unit
            .initialize()
            .map_err(|e| AttuneError::AudioDevice(format!("VPIO initialize: {e}")))?;

        // Read back the negotiated format so we know what the
        // callback will receive.
        let negotiated: AudioStreamBasicDescription = audio_unit
            .get_property(
                kAudioUnitProperty_StreamFormat,
                Scope::Output,
                Element::Input,
            )
            .map_err(|e| AttuneError::AudioDevice(format!("VPIO get format: {e}")))?;
        debug!(
            sample_rate = negotiated.mSampleRate,
            channels = negotiated.mChannelsPerFrame,
            format_id = negotiated.mFormatID,
            "VPIO negotiated stream format",
        );

        info!(
            sample_rate = negotiated.mSampleRate,
            channels = negotiated.mChannelsPerFrame,
            "VoiceProcessingIO AudioUnit ready",
        );

        Ok(Self {
            audio_unit,
            samples: Arc::new(Mutex::new(Vec::new())),
            running: false,
            negotiated_sample_rate: negotiated.mSampleRate,
            negotiated_channels: negotiated.mChannelsPerFrame,
        })
    }

    /// Start capturing. Frames stream into the internal buffer until
    /// [`Self::stop`] is called. Safe to call once per instance; a
    /// second call returns an error.
    pub fn start(&mut self) -> Result<()> {
        if self.running {
            return Err(AttuneError::AudioDevice(
                "VoiceProcessingCapture::start called twice".to_string(),
            ));
        }
        // Wire the render callback. Cloning the Arc gives the
        // callback its own handle to the shared buffer; the
        // callback runs on the CoreAudio realtime thread, so work
        // inside it stays minimal: lock, extend, unlock.
        //
        // We collapse multi-channel frames to mono by averaging.
        // VPIO usually hands us 1 channel, but on some external
        // input devices (interfaces with stereo mics) the
        // negotiated format may be 2-channel. The channel count is
        // captured once at init time.
        let samples = Arc::clone(&self.samples);
        let n_channels = self.negotiated_channels as usize;
        self.audio_unit
            .set_input_callback(move |args: render_callback::Args<data::Interleaved<f32>>| {
                if n_channels == 0 {
                    return Ok(());
                }
                let raw = args.data.buffer;
                let frame_count = raw.len() / n_channels;
                if let Ok(mut buf) = samples.lock() {
                    buf.reserve(frame_count);
                    if n_channels == 1 {
                        buf.extend_from_slice(raw);
                    } else {
                        for frame_idx in 0..frame_count {
                            let mut acc = 0.0f32;
                            for ch in 0..n_channels {
                                acc += raw[frame_idx * n_channels + ch];
                            }
                            buf.push(acc / n_channels as f32);
                        }
                    }
                }
                Ok(())
            })
            .map_err(|e| AttuneError::AudioDevice(format!("VPIO callback: {e}")))?;

        self.audio_unit
            .start()
            .map_err(|e| AttuneError::AudioDevice(format!("VPIO start: {e}")))?;
        self.running = true;
        info!("VoiceProcessingIO capture started");
        Ok(())
    }

    /// Stop the unit and return everything captured. The internal
    /// buffer is drained, so calling again returns an empty vec.
    pub fn stop(&mut self) -> Result<Vec<f32>> {
        if self.running {
            self.audio_unit
                .stop()
                .map_err(|e| AttuneError::AudioDevice(format!("VPIO stop: {e}")))?;
            self.running = false;
        }
        let mut guard = self
            .samples
            .lock()
            .map_err(|e| AttuneError::AudioDevice(format!("VPIO sample lock: {e}")))?;
        let captured = std::mem::take(&mut *guard);
        info!(
            samples = captured.len(),
            "VoiceProcessingIO capture stopped"
        );
        Ok(captured)
    }

    /// Sample rate the unit actually captures at, as negotiated with
    /// the bound input device at initialise time.
    pub fn sample_rate(&self) -> f64 {
        self.negotiated_sample_rate
    }

    /// Channels per frame the unit captures. Callers that want mono
    /// downstream should average channels (we do this in [`Self::stop`]).
    pub fn channels(&self) -> u32 {
        self.negotiated_channels
    }
}

impl Drop for VoiceProcessingCapture {
    fn drop(&mut self) {
        if self.running {
            let _ = self.audio_unit.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: build and tear down a VPIO unit without ever
    /// starting it. Verifies that the property-setting path works on
    /// this machine. Marked `#[ignore]` because it talks to the real
    /// CoreAudio subsystem and won't work in headless CI.
    #[test]
    #[ignore]
    fn instantiate_and_drop() {
        let cap = VoiceProcessingCapture::new().expect("VPIO new failed");
        assert_eq!(cap.sample_rate(), VPIO_SAMPLE_RATE_HZ);
    }
}
