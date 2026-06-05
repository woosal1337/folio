use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use coreaudio::audio_unit::render_callback::{self, data};
use coreaudio::audio_unit::{AudioUnit, Element, IOType, Scope};
use coreaudio_sys::{
    kAudioOutputUnitProperty_EnableIO, kAudioUnitProperty_StreamFormat, AudioStreamBasicDescription,
};
use parking_lot::Mutex as PlMutex;
use tracing::{debug, error, info, warn};

use super::ducking::apply_minimum_ducking;
use crate::audio::resampler::StreamingResampler;
use crate::audio::wav_writer::AudioWavWriter;
use crate::error::{AttuneError, Result};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

const VPIO_SILENCE_WARN_SECS: u64 = 5;

pub struct VoiceProcessingMicCapture {
    audio_unit: AudioUnit,

    _writer: Arc<AudioWavWriter>,
    running: bool,

    input_sample_rate: u32,

    input_channels: u32,

    pub last_callback_ms: Arc<AtomicU64>,

    pub received_audio: Arc<AtomicBool>,

    started_ms: u64,
}

impl VoiceProcessingMicCapture {
    pub fn start(writer: Arc<AudioWavWriter>, target_sample_rate: u32) -> Result<Self> {
        debug!("starting streaming VPIO mic capture");
        let mut audio_unit = AudioUnit::new_uninitialized(IOType::VoiceProcessingIO)
            .map_err(|e| AttuneError::AudioDevice(format!("VPIO instantiate: {e}")))?;

        let disable: u32 = 0;
        if let Err(e) = audio_unit.set_property(
            kAudioOutputUnitProperty_EnableIO,
            Scope::Output,
            Element::Output,
            Some(&disable),
        ) {
            warn!(error = %e, "VPIO disable output failed (non-fatal) — continuing");
        }

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

        let resampler = Arc::new(PlMutex::new(StreamingResampler::new(
            input_sample_rate,
            1,
            target_sample_rate,
        )?));

        let writer_for_cb = Arc::clone(&writer);
        let resampler_for_cb = Arc::clone(&resampler);
        let n_channels = input_channels as usize;

        let last_callback_ms: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
        let received_audio: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let last_cb_for_cb = Arc::clone(&last_callback_ms);
        let received_for_cb = Arc::clone(&received_audio);

        let mono_scratch: Arc<PlMutex<Vec<f32>>> = Arc::new(PlMutex::new(Vec::with_capacity(4096)));

        audio_unit
            .set_input_callback(move |args: render_callback::Args<data::Interleaved<f32>>| {
                if n_channels == 0 {
                    return Ok(());
                }
                let raw = args.data.buffer;
                let frame_count = raw.len() / n_channels;

                last_cb_for_cb.store(now_ms(), Ordering::Relaxed);
                if frame_count > 0 {
                    received_for_cb.store(true, Ordering::Relaxed);
                }

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

        let started_ms = now_ms();
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
            last_callback_ms,
            received_audio,
            started_ms,
        })
    }

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

    pub fn is_silent(&self) -> bool {
        if self.received_audio.load(Ordering::Relaxed) {
            return false;
        }
        let elapsed_ms = now_ms().saturating_sub(self.started_ms);
        elapsed_ms >= VPIO_SILENCE_WARN_SECS * 1000
    }
}

impl Drop for VoiceProcessingMicCapture {
    fn drop(&mut self) {
        if self.running {
            let _ = self.audio_unit.stop();
        }
    }
}
