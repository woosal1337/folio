use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{bounded, Receiver, Sender};
use tracing::{debug, info, warn};

use crate::error::{AttuneError, Result};

const RING_FRAMES: usize = 4096;

pub struct MicMonitor {
    _input: cpal::Stream,
    _output: cpal::Stream,
    stopped: Arc<AtomicBool>,
}

unsafe impl Send for MicMonitor {}

impl MicMonitor {
    pub fn start(device_name: Option<&str>) -> Result<Self> {
        let host = cpal::default_host();

        let input_dev = if let Some(name) = device_name {
            host.input_devices()
                .map_err(|e| AttuneError::AudioDevice(format!("input_devices: {e}")))?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                .or_else(|| host.default_input_device())
        } else {
            host.default_input_device()
        }
        .ok_or(AttuneError::NoInputDevice)?;

        let output_dev = host
            .default_output_device()
            .ok_or_else(|| AttuneError::AudioDevice("no default output device".into()))?;

        let in_cfg = input_dev
            .default_input_config()
            .map_err(|e| AttuneError::AudioDevice(format!("input config: {e}")))?;
        let out_cfg = output_dev
            .default_output_config()
            .map_err(|e| AttuneError::AudioDevice(format!("output config: {e}")))?;

        let in_channels = in_cfg.channels() as usize;
        let out_channels = out_cfg.channels() as usize;

        let (tx, rx): (Sender<f32>, Receiver<f32>) = bounded(RING_FRAMES);
        let tx2 = tx.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let stopped_for_in = Arc::clone(&stopped);
        let stopped_for_out = Arc::clone(&stopped);

        let input_stream = input_dev
            .build_input_stream(
                &in_cfg.into(),
                move |data: &[f32], _| {
                    if stopped_for_in.load(Ordering::Relaxed) {
                        return;
                    }
                    for frame in data.chunks(in_channels.max(1)) {
                        let mono = frame.iter().sum::<f32>() / in_channels as f32;

                        let _ = tx.try_send(mono);
                    }
                },
                |e| warn!(error = %e, "mic_monitor input error"),
                None,
            )
            .map_err(|e| AttuneError::AudioDevice(format!("build input stream: {e}")))?;

        let output_stream = output_dev
            .build_output_stream(
                &out_cfg.into(),
                move |data: &mut [f32], _| {
                    if stopped_for_out.load(Ordering::Relaxed) {
                        data.fill(0.0);
                        return;
                    }
                    let frames = data.len() / out_channels.max(1);
                    for f in 0..frames {
                        let sample = rx.try_recv().unwrap_or(0.0);
                        for c in 0..out_channels.max(1) {
                            data[f * out_channels.max(1) + c] = sample;
                        }
                    }
                },
                |e| warn!(error = %e, "mic_monitor output error"),
                None,
            )
            .map_err(|e| AttuneError::AudioDevice(format!("build output stream: {e}")))?;

        input_stream
            .play()
            .map_err(|e| AttuneError::AudioDevice(format!("input stream play: {e}")))?;
        output_stream
            .play()
            .map_err(|e| AttuneError::AudioDevice(format!("output stream play: {e}")))?;

        info!("mic monitor started");
        let _ = tx2;

        Ok(Self {
            _input: input_stream,
            _output: output_stream,
            stopped,
        })
    }

    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
        debug!("mic monitor stopped");
    }
}

impl Drop for MicMonitor {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
    }
}
