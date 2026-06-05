#![allow(non_snake_case, non_upper_case_globals)]

use std::ffi::CStr;
use std::sync::Arc;

use coreaudio::audio_unit::{AudioUnit, Element, IOType, Scope};
use coreaudio_sys::{
    kAudioObjectPropertyScopeGlobal, kAudioOutputUnitProperty_EnableIO, AudioObjectGetPropertyData,
    AudioObjectID, AudioObjectPropertyAddress, AudioObjectPropertySelector,
    AudioStreamBasicDescription,
};
use objc2_core_audio::{
    AudioHardwareCreateProcessTap, AudioHardwareDestroyProcessTap, CATapDescription,
};
use parking_lot::Mutex;
use tracing::{debug, error, info, warn};

use crate::audio::resampler::StreamingResampler;
use crate::audio::wav_writer::AudioWavWriter;
use crate::error::{FolioError, Result};

const MIN_MAJOR: u32 = 14;
const MIN_MINOR: u32 = 4;

pub fn is_supported() -> bool {
    let mut major: u32 = 0;
    let mut minor: u32 = 0;

    let major_ok = get_os_release_component("kern.osproductversion", &mut major, &mut minor);
    if !major_ok {
        return false;
    }
    major > MIN_MAJOR || (major == MIN_MAJOR && minor >= MIN_MINOR)
}

fn get_os_release_component(key: &str, major: &mut u32, minor: &mut u32) -> bool {
    use std::ffi::CString;

    let c_key = match CString::new(key) {
        Ok(k) => k,
        Err(_) => return false,
    };
    let mut buf = [0u8; 64];
    let mut len: libc::size_t = buf.len();
    let ret = unsafe {
        libc::sysctlbyname(
            c_key.as_ptr(),
            buf.as_mut_ptr() as *mut libc::c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if ret != 0 || len == 0 {
        return false;
    }
    let s = match CStr::from_bytes_until_nul(&buf[..len]) {
        Ok(s) => s.to_string_lossy(),
        Err(_) => return false,
    };

    let mut parts = s.splitn(3, '.');
    *major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    *minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    *major > 0
}

const kAudioTapPropertyFormat: AudioObjectPropertySelector = 0x74666d74;

fn global_address(selector: AudioObjectPropertySelector) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: 0,
    }
}

pub struct ProcessTapCapture {
    audio_unit: AudioUnit,
    writer: Arc<AudioWavWriter>,
    tap_id: AudioObjectID,
}

unsafe impl Send for ProcessTapCapture {}

impl ProcessTapCapture {
    pub fn start(writer: Arc<AudioWavWriter>, target_sample_rate: u32) -> Result<Self> {
        if !is_supported() {
            return Err(FolioError::SystemAudio(
                "CoreAudio process tap requires macOS 14.4+".into(),
            ));
        }

        let tap_id = Self::create_tap()?;
        debug!(tap_id, "process tap created");

        let tap_format = Self::read_tap_format(tap_id)?;
        let tap_rate = tap_format.mSampleRate.round() as u32;
        info!(
            tap_id,
            sample_rate = tap_rate,
            channels = tap_format.mChannelsPerFrame,
            "process tap format negotiated"
        );

        let audio_unit = Self::open_auhal_on_tap(tap_id)?;

        let resampler = Arc::new(Mutex::new(StreamingResampler::new(
            tap_rate,
            1,
            target_sample_rate,
        )?));
        let writer_for_cb = Arc::clone(&writer);
        let resampler_for_cb = Arc::clone(&resampler);
        let n_channels = tap_format.mChannelsPerFrame as usize;

        let mono_scratch: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(4096)));

        let mut unit = audio_unit;
        unit.set_input_callback(
            move |args: coreaudio::audio_unit::render_callback::Args<
                coreaudio::audio_unit::render_callback::data::Interleaved<f32>,
            >| {
                let raw = args.data.buffer;
                if raw.is_empty() {
                    return Ok(());
                }
                let ch = n_channels.max(1);
                let frames = raw.len() / ch;
                let mut mono = mono_scratch.lock();
                mono.clear();
                mono.reserve(frames);
                if ch == 1 {
                    mono.extend_from_slice(raw);
                } else {
                    for f in 0..frames {
                        let mut acc = 0.0f32;
                        for c in 0..ch {
                            acc += raw[f * ch + c];
                        }
                        mono.push(acc / ch as f32);
                    }
                }
                let mut rs = resampler_for_cb.lock();
                match rs.process(&mono) {
                    Ok(out) => {
                        if let Err(e) = writer_for_cb.append(&out) {
                            error!(error = %e, "process-tap WAV write failed");
                        }
                    }
                    Err(e) => error!(error = %e, "process-tap resampler failed"),
                }
                Ok(())
            },
        )
        .map_err(|e| FolioError::SystemAudio(format!("process-tap callback: {e}")))?;

        unit.start()
            .map_err(|e| FolioError::SystemAudio(format!("process-tap AUHAL start: {e}")))?;

        info!(
            tap_id,
            target_sample_rate, "process-tap system audio started"
        );
        Ok(Self {
            audio_unit: unit,
            writer,
            tap_id,
        })
    }

    pub fn stop(mut self) -> Result<()> {
        if let Err(e) = self.audio_unit.stop() {
            warn!(error = %e, "process-tap AUHAL stop error (non-fatal)");
        }

        std::thread::sleep(std::time::Duration::from_millis(150));

        let status = unsafe { AudioHardwareDestroyProcessTap(self.tap_id) };
        if status != 0 {
            warn!(
                tap_id = self.tap_id,
                status, "AudioHardwareDestroyProcessTap non-zero status"
            );
        }
        self.writer.finalize()?;
        info!(tap_id = self.tap_id, "process-tap system audio stopped");
        Ok(())
    }

    fn create_tap() -> Result<AudioObjectID> {
        let tap_desc = unsafe { CATapDescription::new() };

        let mut tap_id: AudioObjectID = 0;
        let status = unsafe { AudioHardwareCreateProcessTap(Some(&tap_desc), &mut tap_id) };
        if status != 0 {
            return Err(FolioError::SystemAudio(format!(
                "AudioHardwareCreateProcessTap failed: OSStatus {status} \
                 (if this is the first launch, the OS may need to prompt for permission)"
            )));
        }
        if tap_id == 0 {
            return Err(FolioError::SystemAudio(
                "AudioHardwareCreateProcessTap returned tap_id = 0".into(),
            ));
        }
        Ok(tap_id)
    }

    fn read_tap_format(tap_id: AudioObjectID) -> Result<AudioStreamBasicDescription> {
        let addr = global_address(kAudioTapPropertyFormat);
        let mut fmt = AudioStreamBasicDescription {
            mSampleRate: 0.0,
            mFormatID: 0,
            mFormatFlags: 0,
            mBytesPerPacket: 0,
            mFramesPerPacket: 0,
            mBytesPerFrame: 0,
            mChannelsPerFrame: 0,
            mBitsPerChannel: 0,
            mReserved: 0,
        };
        let mut size = std::mem::size_of::<AudioStreamBasicDescription>() as u32;
        let status = unsafe {
            AudioObjectGetPropertyData(
                tap_id,
                &addr,
                0,
                std::ptr::null(),
                &mut size,
                &mut fmt as *mut _ as *mut _,
            )
        };
        if status != 0 {
            return Err(FolioError::SystemAudio(format!(
                "kAudioTapPropertyFormat read failed: OSStatus {status}"
            )));
        }

        if fmt.mSampleRate < 1.0 {
            fmt.mSampleRate = 48_000.0;
            fmt.mChannelsPerFrame = 2;
        }
        Ok(fmt)
    }

    fn open_auhal_on_tap(tap_id: AudioObjectID) -> Result<AudioUnit> {
        let mut unit = AudioUnit::new_uninitialized(IOType::HalOutput)
            .map_err(|e| FolioError::SystemAudio(format!("AUHAL new: {e}")))?;

        let off: u32 = 0;
        unit.set_property(
            kAudioOutputUnitProperty_EnableIO,
            Scope::Output,
            Element::Output,
            Some(&off),
        )
        .map_err(|e| FolioError::SystemAudio(format!("AUHAL disable output: {e}")))?;

        let on: u32 = 1;
        unit.set_property(
            kAudioOutputUnitProperty_EnableIO,
            Scope::Input,
            Element::Input,
            Some(&on),
        )
        .map_err(|e| FolioError::SystemAudio(format!("AUHAL enable input: {e}")))?;

        const kAudioOutputUnitProperty_CurrentDevice: u32 = 2000;
        unit.set_property(
            kAudioOutputUnitProperty_CurrentDevice,
            Scope::Global,
            Element::Output,
            Some(&tap_id),
        )
        .map_err(|e| FolioError::SystemAudio(format!("AUHAL bind tap: {e}")))?;

        unit.initialize()
            .map_err(|e| FolioError::SystemAudio(format!("AUHAL initialize: {e}")))?;

        Ok(unit)
    }
}

impl Drop for ProcessTapCapture {
    fn drop(&mut self) {
        let _ = self.audio_unit.stop();
        unsafe { AudioHardwareDestroyProcessTap(self.tap_id) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_supported_does_not_panic() {
        let _ = is_supported();
    }

    #[test]
    fn global_address_has_correct_scope() {
        let addr = global_address(kAudioTapPropertyFormat);
        assert_eq!(addr.mScope, kAudioObjectPropertyScopeGlobal);
        assert_eq!(addr.mSelector, kAudioTapPropertyFormat);
    }
}
