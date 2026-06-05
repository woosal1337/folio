use std::sync::{Arc, Mutex};

use coreaudio::audio_unit::render_callback::{self, data};
use coreaudio::audio_unit::{AudioUnit, Element, IOType, Scope};
use coreaudio_sys::{
    kAudioOutputUnitProperty_EnableIO, kAudioUnitProperty_StreamFormat, AudioStreamBasicDescription,
};
use tracing::{debug, info};

use super::ducking::apply_minimum_ducking;
use crate::error::{AttuneError, Result};

pub struct VoiceProcessingCapture {
    audio_unit: AudioUnit,
    samples: Arc<Mutex<Vec<f32>>>,
    running: bool,

    negotiated_sample_rate: f64,

    negotiated_channels: u32,
}

impl VoiceProcessingCapture {
    pub fn new() -> Result<Self> {
        debug!("instantiating VoiceProcessingIO AudioUnit");
        let mut audio_unit = AudioUnit::new_uninitialized(IOType::VoiceProcessingIO)
            .map_err(|e| AttuneError::AudioDevice(format!("VPIO instantiate: {e}")))?;

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

    pub fn start(&mut self) -> Result<()> {
        if self.running {
            return Err(AttuneError::AudioDevice(
                "VoiceProcessingCapture::start called twice".to_string(),
            ));
        }

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

    pub fn sample_rate(&self) -> f64 {
        self.negotiated_sample_rate
    }

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
