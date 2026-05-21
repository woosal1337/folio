//! Audio device enumeration. Used by the GUI and CLI to let the user pick
//! a mic. Falls back to the default device when no name is specified.

use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

use crate::error::{AttuneError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
                eprintln!("no input devices on this machine: {e}");
            }
        }
    }
}
