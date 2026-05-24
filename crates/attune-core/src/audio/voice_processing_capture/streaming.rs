//! Streaming VPIO mic capture used by the Tauri app's
//! [`crate::audio::CaptureSession`]. Drop-in replacement for
//! [`crate::audio::mic::MicCapture`] when voice processing is enabled.
//!
//! Owns the AudioUnit + the shared writer Arc + the resampler. The
//! render callback runs on the CoreAudio realtime thread; it averages
//! multi-channel frames to mono, resamples to the writer's target
//! rate, and writes — same contract as the cpal mic callback.

use std::sync::Arc;

use coreaudio::audio_unit::render_callback::{self, data};
use coreaudio::audio_unit::{AudioUnit, Element, IOType, Scope};
use coreaudio_sys::{
    kAudioOutputUnitProperty_EnableIO, kAudioUnitProperty_StreamFormat, AudioStreamBasicDescription,
};
use parking_lot::Mutex as PlMutex;
use tracing::{debug, error, info};

use super::ducking::apply_minimum_ducking;
use crate::audio::resampler::StreamingResampler;
use crate::audio::wav_writer::AudioWavWriter;
use crate::error::{AttuneError, Result};

/// Production VPIO mic capture that writes directly into an
/// [`AudioWavWriter`] via [`StreamingResampler`].
pub struct VoiceProcessingMicCapture {
    audio_unit: AudioUnit,
    /// Held so the WAV stays open for the unit's lifetime; the
    /// callback writes into a clone of this Arc.
    _writer: Arc<AudioWavWriter>,
    running: bool,
    /// Sample rate the unit captures at (negotiated with the device,
    /// usually 44.1 kHz on built-in M-series mics).
    input_sample_rate: u32,
    /// Channels per frame the unit emits. Usually 1; can be 2 on some
    /// USB interfaces.
    input_channels: u32,
}

impl VoiceProcessingMicCapture {
    /// Build, configure, and start a VPIO unit that writes to `writer`.
    /// Returns once the unit is running.
    ///
    /// `target_sample_rate` is the rate the WAV is written at. The
    /// callback resamples from VPIO's negotiated input rate into
    /// `target_sample_rate` before handing samples to the writer.
    pub fn start(writer: Arc<AudioWavWriter>, target_sample_rate: u32) -> Result<Self> {
        debug!("starting streaming VPIO mic capture");
        let mut audio_unit = AudioUnit::new_uninitialized(IOType::VoiceProcessingIO)
            .map_err(|e| AttuneError::AudioDevice(format!("VPIO instantiate: {e}")))?;

        // Same minimal setup as the buffered variant: enable input
        // bus 1, init, read back the negotiated format. See
        // `super::buffered` for the rationale on the minimal config.
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

        apply_minimum_ducking(&mut audio_unit);

        let negotiated: AudioStreamBasicDescription = audio_unit
            .get_property(
                kAudioUnitProperty_StreamFormat,
                Scope::Output,
                Element::Input,
            )
            .map_err(|e| AttuneError::AudioDevice(format!("VPIO get format: {e}")))?;
        let input_sample_rate = negotiated.mSampleRate.round() as u32;
        let input_channels = negotiated.mChannelsPerFrame;
        info!(
            input_sample_rate,
            input_channels, target_sample_rate, "VPIO streaming mic capture ready",
        );

        // Resampler converts VPIO's negotiated rate to the WAV writer's
        // target rate. Mono input + mono output — we collapse
        // multi-channel input to mono before the resampler sees it.
        let resampler = Arc::new(PlMutex::new(StreamingResampler::new(
            input_sample_rate,
            1,
            target_sample_rate,
        )?));

        let writer_for_cb = Arc::clone(&writer);
        let resampler_for_cb = Arc::clone(&resampler);
        let n_channels = input_channels as usize;

        // Reusable scratch buffer for the mono-fold step. Allocated
        // once outside the closure and captured by move. The callback
        // pushes into it, hands a slice to the resampler, and clears.
        // Avoids per-callback allocation on the realtime thread.
        let mono_scratch: Arc<PlMutex<Vec<f32>>> = Arc::new(PlMutex::new(Vec::with_capacity(4096)));

        audio_unit
            .set_input_callback(move |args: render_callback::Args<data::Interleaved<f32>>| {
                if n_channels == 0 {
                    return Ok(());
                }
                let raw = args.data.buffer;
                let frame_count = raw.len() / n_channels;

                let mut mono = mono_scratch.lock();
                mono.clear();
                mono.reserve(frame_count);
                if n_channels == 1 {
                    mono.extend_from_slice(raw);
                } else {
                    for frame_idx in 0..frame_count {
                        let mut acc = 0.0f32;
                        for ch in 0..n_channels {
                            acc += raw[frame_idx * n_channels + ch];
                        }
                        mono.push(acc / n_channels as f32);
                    }
                }

                // Resample + write. On any failure we log once and
                // keep the callback returning Ok so the unit doesn't
                // tear itself down mid-recording.
                let mut resampler = resampler_for_cb.lock();
                match resampler.process(&mono) {
                    Ok(resampled) => {
                        if let Err(e) = writer_for_cb.append(&resampled) {
                            error!(error = %e, "VPIO writer failed");
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "VPIO resampler failed");
                    }
                }
                Ok(())
            })
            .map_err(|e| AttuneError::AudioDevice(format!("VPIO callback: {e}")))?;

        audio_unit
            .start()
            .map_err(|e| AttuneError::AudioDevice(format!("VPIO start: {e}")))?;
        info!("VPIO streaming mic capture started");

        Ok(Self {
            audio_unit,
            _writer: writer,
            running: true,
            input_sample_rate,
            input_channels,
        })
    }

    /// Stop the unit. The WAV writer is held by the caller via Arc;
    /// finalisation happens when the caller drops their last
    /// reference to the writer.
    pub fn stop(mut self) -> Result<()> {
        if self.running {
            self.audio_unit
                .stop()
                .map_err(|e| AttuneError::AudioDevice(format!("VPIO stop: {e}")))?;
            self.running = false;
            info!("VPIO streaming mic capture stopped");
        }
        Ok(())
    }

    pub fn input_sample_rate(&self) -> u32 {
        self.input_sample_rate
    }

    pub fn input_channels(&self) -> u32 {
        self.input_channels
    }
}

impl Drop for VoiceProcessingMicCapture {
    fn drop(&mut self) {
        if self.running {
            let _ = self.audio_unit.stop();
        }
    }
}
