//! Audio device enumeration. Used by the GUI and CLI to let the user pick
//! a mic. Falls back to the default device when no name is specified.

use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AttuneError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct DeviceInfo {
    pub name: String,
    pub is_default: bool,
    /// Default input sample rate reported by the device. May be `None` if
    /// the device cannot be queried (rare).
    pub default_sample_rate: Option<u32>,
    /// Default input channel count reported by the device.
    pub default_channels: Option<u16>,
}

/// Query the native default sample rate of a specific input device, or the
/// system default if `name` is `None`. Returned in Hz.
pub fn default_input_sample_rate(name: Option<&str>) -> Result<u32> {
    let host = cpal::default_host();
    let device = match name {
        Some(name) => host
            .input_devices()
            .map_err(|e| AttuneError::AudioDevice(format!("input_devices: {e}")))?
            .find(|d| d.name().ok().as_deref() == Some(name))
            .ok_or_else(|| AttuneError::AudioDevice(format!("input device not found: {name}")))?,
        None => host
            .default_input_device()
            .ok_or(AttuneError::NoInputDevice)?,
    };
    let cfg = device
        .default_input_config()
        .map_err(|e| AttuneError::AudioDevice(format!("default_input_config: {e}")))?;
    Ok(cfg.sample_rate().0)
}

/// List all input devices visible to the default audio host. The default
/// device, if any, is marked with `is_default = true`.
pub fn list_input_devices() -> Result<Vec<DeviceInfo>> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|d| d.name().ok());

    let devices_iter = host
        .input_devices()
        .map_err(|e| AttuneError::AudioDevice(format!("input_devices: {e}")))?;

    let mut out = Vec::new();
    for device in devices_iter {
        let name = device.name().unwrap_or_else(|_| "<unknown>".to_string());
        let cfg = device.default_input_config().ok();
        let (sr, ch) = match cfg {
            Some(c) => (Some(c.sample_rate().0), Some(c.channels())),
            None => (None, None),
        };
        let is_default = default_name.as_deref() == Some(name.as_str());
        out.push(DeviceInfo {
            name,
            is_default,
            default_sample_rate: sr,
            default_channels: ch,
        });
    }
    // Default first, then alphabetical.
    out.sort_by(|a, b| match (a.is_default, b.is_default) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });
    Ok(out)
}

// ---------------------------------------------------------------------------
// Pre-flight mic level check (GET-212)
// ---------------------------------------------------------------------------

/// Mic level status derived from a brief RMS measurement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicStatus {
    /// RMS is in the usable range (>= −48 dBFS, < −3 dBFS peak).
    Ok,
    /// RMS is below −48 dBFS — mic gain is likely too low.
    TooQuiet,
    /// Peak is >= −3 dBFS — input is at risk of clipping.
    Clipping,
}

/// Result of a brief mic input-level sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicLevelResult {
    /// RMS level in dBFS (negative; 0 = full scale).
    pub rms_db: f32,
    /// Peak level in dBFS.
    pub peak_db: f32,
    /// Qualitative status.
    pub status: MicStatus,
    /// Deep-link the user can open to raise mic gain in System Settings.
    pub settings_url: String,
}

/// Sample the input device for `duration_ms` milliseconds and return
/// the peak + RMS level in dBFS.
///
/// # Errors
///
/// Returns `Err` if the input device cannot be opened or the stream
/// fails to start.
pub fn sample_mic_level(device_name: Option<&str>, duration_ms: u64) -> Result<MicLevelResult> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    let host = cpal::default_host();
    let device = if let Some(name) = device_name {
        host.input_devices()
            .map_err(|e| AttuneError::AudioDevice(format!("input_devices: {e}")))?
            .find(|d| d.name().map(|n| n == name).unwrap_or(false))
            .or_else(|| host.default_input_device())
    } else {
        host.default_input_device()
    }
    .ok_or(AttuneError::NoInputDevice)?;

    let config = device
        .default_input_config()
        .map_err(|e| AttuneError::AudioDevice(format!("default_input_config: {e}")))?;
    let channels = config.channels() as usize;

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_cb = Arc::clone(&samples);

    let stream = device
        .build_input_stream(
            &config.into(),
            move |data: &[f32], _| {
                if let Ok(mut buf) = samples_cb.lock() {
                    for frame in data.chunks(channels.max(1)) {
                        let mono = frame.iter().copied().sum::<f32>() / channels as f32;
                        buf.push(mono);
                    }
                }
            },
            |e| tracing::warn!(error = %e, "mic level check stream error"),
            None,
        )
        .map_err(|e| AttuneError::AudioDevice(format!("build_input_stream: {e}")))?;

    stream
        .play()
        .map_err(|e| AttuneError::AudioDevice(format!("stream.play: {e}")))?;
    std::thread::sleep(Duration::from_millis(duration_ms));
    drop(stream);

    let buf = samples.lock().unwrap();
    let to_db = |x: f32| {
        if x < 1e-9 {
            -96.0_f32
        } else {
            20.0 * x.log10()
        }
    };

    if buf.is_empty() {
        return Ok(MicLevelResult {
            rms_db: -96.0,
            peak_db: -96.0,
            status: MicStatus::TooQuiet,
            settings_url: "x-apple.systempreferences:com.apple.preference.sound?input".into(),
        });
    }

    let rms = (buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32).sqrt();
    let peak = buf.iter().copied().map(|s| s.abs()).fold(0.0_f32, f32::max);
    let rms_db = to_db(rms);
    let peak_db = to_db(peak);

    let status = if peak_db >= -3.0 {
        MicStatus::Clipping
    } else if rms_db < -48.0 {
        MicStatus::TooQuiet
    } else {
        MicStatus::Ok
    };

    Ok(MicLevelResult {
        rms_db,
        peak_db,
        status,
        settings_url: "x-apple.systempreferences:com.apple.preference.sound?input".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test. CI Mac runners may not have an input device; skip when
    /// none is present.
    #[test]
    fn list_input_devices_runs() {
        let result = list_input_devices();
        match result {
            Ok(devices) => {
                for d in devices {
                    assert!(!d.name.is_empty());
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "no input devices on this machine");
            }
        }
    }
}
