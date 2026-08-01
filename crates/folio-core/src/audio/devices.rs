use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{FolioError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct DeviceInfo {
    pub name: String,
    pub is_default: bool,

    pub default_sample_rate: Option<u32>,

    pub default_channels: Option<u16>,
}

pub fn default_input_sample_rate(name: Option<&str>) -> Result<u32> {
    let host = cpal::default_host();
    let device = match name {
        Some(name) => host
            .input_devices()
            .map_err(|e| FolioError::AudioDevice(format!("input_devices: {e}")))?
            .find(|d| d.name().ok().as_deref() == Some(name))
            .ok_or_else(|| FolioError::AudioDevice(format!("input device not found: {name}")))?,
        None => host
            .default_input_device()
            .ok_or(FolioError::NoInputDevice)?,
    };
    let cfg = device
        .default_input_config()
        .map_err(|e| FolioError::AudioDevice(format!("default_input_config: {e}")))?;
    Ok(cfg.sample_rate().0)
}

pub fn list_input_devices() -> Result<Vec<DeviceInfo>> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|d| d.name().ok());

    let devices_iter = host
        .input_devices()
        .map_err(|e| FolioError::AudioDevice(format!("input_devices: {e}")))?;

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

    out.sort_by(|a, b| match (a.is_default, b.is_default) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicStatus {
    Ok,

    TooQuiet,

    Clipping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicLevelResult {
    pub rms_db: f32,

    pub peak_db: f32,

    pub status: MicStatus,

    pub settings_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

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
