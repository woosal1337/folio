use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat, Stream, StreamConfig};
use parking_lot::Mutex;
use tracing::{debug, error, info, warn};

use crate::audio::level_meter::LevelMeter;
use crate::audio::resampler::StreamingResampler;
use crate::audio::wav_writer::AudioWavWriter;
use crate::error::{FolioError, Result};
use crate::qos::{set_thread_qos, QosClass};

thread_local! {



    static QOS_TAGGED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn ensure_capture_qos() {
    QOS_TAGGED.with(|cell| {
        if !cell.get() {
            set_thread_qos(QosClass::UserInteractive);
            cell.set(true);
        }
    });
}

pub struct MicCapture {
    stream: Option<Stream>,
    writer: Arc<AudioWavWriter>,
    stopped: Arc<AtomicBool>,
    level: Arc<LevelMeter>,
    input_sample_rate: u32,
    input_channels: u16,
}

impl MicCapture {
    pub fn start(
        writer: Arc<AudioWavWriter>,
        target_sample_rate: u32,
        device_name: Option<&str>,
    ) -> Result<Self> {
        let host = cpal::default_host();
        let device = match device_name {
            Some(name) => host
                .input_devices()
                .map_err(|e| FolioError::AudioDevice(format!("input_devices: {e}")))?
                .find(|d| d.name().ok().as_deref() == Some(name))
                .ok_or_else(|| {
                    FolioError::AudioDevice(format!("input device not found: {name}"))
                })?,
            None => host
                .default_input_device()
                .ok_or(FolioError::NoInputDevice)?,
        };

        let device_name = device.name().unwrap_or_else(|_| "<unknown>".to_string());
        info!(device = %device_name, "selected default input device");

        let supported_config = device
            .default_input_config()
            .map_err(|e| FolioError::AudioDevice(format!("default_input_config: {e}")))?;
        let sample_format = supported_config.sample_format();
        let input_sample_rate = supported_config.sample_rate().0;
        let input_channels = supported_config.channels();
        let config: StreamConfig = supported_config.into();

        info!(
            sample_rate = input_sample_rate,
            channels = input_channels,
            ?sample_format,
            "mic capture config",
        );

        let resampler = Arc::new(Mutex::new(StreamingResampler::new(
            input_sample_rate,
            input_channels,
            target_sample_rate,
        )?));

        let stopped = Arc::new(AtomicBool::new(false));
        let level = Arc::new(LevelMeter::new());
        let stream = build_input_stream(
            &device,
            &config,
            sample_format,
            writer.clone(),
            resampler.clone(),
            stopped.clone(),
            level.clone(),
        )?;

        stream
            .play()
            .map_err(|e| FolioError::StreamPlay(format!("mic stream play: {e}")))?;

        Ok(Self {
            stream: Some(stream),
            writer,
            stopped,
            level,
            input_sample_rate,
            input_channels,
        })
    }

    pub fn input_sample_rate(&self) -> u32 {
        self.input_sample_rate
    }
    pub fn input_channels(&self) -> u16 {
        self.input_channels
    }

    pub fn is_silent(&self) -> bool {
        self.level.is_silent()
    }

    pub fn stop(mut self) -> Result<()> {
        self.stopped.store(true, Ordering::SeqCst);
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }
        self.writer.finalize()?;
        debug!(
            samples = self.writer.samples_written(),
            "mic capture finalized"
        );
        Ok(())
    }
}

fn build_input_stream(
    device: &Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    writer: Arc<AudioWavWriter>,
    resampler: Arc<Mutex<StreamingResampler>>,
    stopped: Arc<AtomicBool>,
    level: Arc<LevelMeter>,
) -> Result<Stream> {
    let err_fn = |err| error!(?err, "mic stream error");
    let stream = match sample_format {
        SampleFormat::F32 => device
            .build_input_stream(
                config,
                move |data: &[f32], _| {
                    handle_samples(data, &writer, &resampler, &stopped, &level);
                },
                err_fn,
                None,
            )
            .map_err(|e| FolioError::StreamBuild(format!("f32 stream: {e}")))?,
        SampleFormat::I16 => {
            let writer = writer.clone();
            let resampler = resampler.clone();
            let stopped = stopped.clone();
            let level = level.clone();
            device
                .build_input_stream(
                    config,
                    move |data: &[i16], _| {
                        let floats: Vec<f32> = data.iter().map(|s| s.to_float_sample()).collect();
                        handle_samples(&floats, &writer, &resampler, &stopped, &level);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| FolioError::StreamBuild(format!("i16 stream: {e}")))?
        }
        SampleFormat::U16 => {
            let writer = writer.clone();
            let resampler = resampler.clone();
            let stopped = stopped.clone();
            let level = level.clone();
            device
                .build_input_stream(
                    config,
                    move |data: &[u16], _| {
                        let floats: Vec<f32> = data.iter().map(|s| s.to_float_sample()).collect();
                        handle_samples(&floats, &writer, &resampler, &stopped, &level);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| FolioError::StreamBuild(format!("u16 stream: {e}")))?
        }
        other => {
            warn!(?other, "unsupported sample format, falling back to f32");
            return Err(FolioError::AudioDevice(format!(
                "unsupported sample format: {other:?}"
            )));
        }
    };
    Ok(stream)
}

fn handle_samples(
    data: &[f32],
    writer: &Arc<AudioWavWriter>,
    resampler: &Arc<Mutex<StreamingResampler>>,
    stopped: &Arc<AtomicBool>,
    level: &Arc<LevelMeter>,
) {
    ensure_capture_qos();
    if stopped.load(Ordering::SeqCst) {
        return;
    }
    let resampled = {
        let mut guard = resampler.lock();
        match guard.process(data) {
            Ok(out) => out,
            Err(e) => {
                error!(error = %e, "resampler process failed");
                return;
            }
        }
    };
    level.record(&resampled);
    if let Err(e) = writer.append(&resampled) {
        error!(error = %e, "wav append failed");
    }
}
