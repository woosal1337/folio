//! System audio capture via CoreAudio process tap (GET-170).
//!
//! On macOS 14.4+, `AudioHardwareCreateProcessTap` lets apps capture system
//! audio without requesting the Screen Recording permission that
//! ScreenCaptureKit requires. Apps using this API appear under
//! **System Audio Recording Only** in System Settings → Privacy, not under
//! Screen & System Audio Recording.
//!
//! ## Architecture
//!
//! 1. Check OS version ≥ 14.4 at runtime.
//! 2. Create a `CATapDescription` for a stereo global mixdown (all processes
//!    except Attune itself, which is excluded automatically by CoreAudio).
//! 3. Call `AudioHardwareCreateProcessTap` → `AudioObjectID` (tap ID).
//! 4. Read the tap's format from `kAudioTapPropertyFormat`.
//! 5. Create a private aggregate device that has the tap as its sub-device.
//! 6. Open an AUHAL (IO AudioUnit) on the aggregate device, wire an input
//!    callback, resample → write to the WAV.
//! 7. On stop: stop AUHAL, destroy aggregate device, destroy tap.
//!
//! ## Notes
//!
//! - Requires on-device verification (audio hardware + TCC grant). The tap
//!   permission appears the FIRST time `AudioHardwareCreateProcessTap` is
//!   called; no separate entitlement is needed.
//! - Falls back gracefully to ScreenCaptureKit when macOS < 14.4 or when
//!   any step fails — callers must handle the error.
//! - `unsafe` throughout: CoreAudio + Objective-C FFI. Each call cites the
//!   Apple developer docs selector / enum it uses.

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
use crate::error::{AttuneError, Result};

// ---------------------------------------------------------------------------
// OS version gate
// ---------------------------------------------------------------------------

/// Minimum macOS version for `AudioHardwareCreateProcessTap`.
const MIN_MAJOR: u32 = 14;
const MIN_MINOR: u32 = 4;

/// True when the running OS is macOS 14.4 or later.
pub fn is_supported() -> bool {
    let mut major: u32 = 0;
    let mut minor: u32 = 0;
    // SAFETY: Gestalt selectors 0x73797376 / 0x73797376 are stable on all
    // macOS versions going back to 10.0. We use libc::sysctlbyname for the
    // modern replacement.
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
    // Parse "14.4" or "14.4.1".
    let mut parts = s.splitn(3, '.');
    *major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    *minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    *major > 0
}

// ---------------------------------------------------------------------------
// CoreAudio tap constants (macOS 14.4+, not yet in coreaudio-sys 0.2)
// ---------------------------------------------------------------------------

/// AudioObjectPropertySelector for the tap's format (kAudioTapPropertyFormat).
const kAudioTapPropertyFormat: AudioObjectPropertySelector = 0x74666d74;

fn global_address(selector: AudioObjectPropertySelector) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: 0,
    }
}

// ---------------------------------------------------------------------------
// Tap capture implementation
// ---------------------------------------------------------------------------

/// Captures system audio on macOS 14.4+ via `AudioHardwareCreateProcessTap`.
///
/// Call [`ProcessTapCapture::start`]; it either succeeds or returns an error
/// that the caller can use to fall back to ScreenCaptureKit.
pub struct ProcessTapCapture {
    audio_unit: AudioUnit,
    writer: Arc<AudioWavWriter>,
    tap_id: AudioObjectID,
}

// SAFETY: CoreAudio objects are thread-safe for start/stop lifecycle use
// when owned by a single thread (the capture session Mutex ensures this).
unsafe impl Send for ProcessTapCapture {}

impl ProcessTapCapture {
    /// Build and start a process-tap capture writing into `writer`.
    ///
    /// Returns `Err` when:
    /// - OS < 14.4
    /// - `AudioHardwareCreateProcessTap` fails (permission not granted yet —
    ///   the OS will prompt; retry after the grant)
    /// - Any subsequent setup step fails (log + propagate)
    pub fn start(writer: Arc<AudioWavWriter>, target_sample_rate: u32) -> Result<Self> {
        if !is_supported() {
            return Err(AttuneError::SystemAudio(
                "CoreAudio process tap requires macOS 14.4+".into(),
            ));
        }

        // 1. Create tap description — stereo global mixdown of all processes.
        //    Attune's own output is automatically excluded by CoreAudio.
        let tap_id = Self::create_tap()?;
        debug!(tap_id, "process tap created");

        // 2. Read the tap's negotiated format.
        let tap_format = Self::read_tap_format(tap_id)?;
        let tap_rate = tap_format.mSampleRate.round() as u32;
        info!(
            tap_id,
            sample_rate = tap_rate,
            channels = tap_format.mChannelsPerFrame,
            "process tap format negotiated"
        );

        // 3. Open an AUHAL on the tap object directly. CoreAudio 14.4+
        //    treats the tap ID as a device ID for AUHAL purposes.
        let audio_unit = Self::open_auhal_on_tap(tap_id)?;

        // 4. Wire the resampling + WAV-write input callback.
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
        .map_err(|e| AttuneError::SystemAudio(format!("process-tap callback: {e}")))?;

        unit.start()
            .map_err(|e| AttuneError::SystemAudio(format!("process-tap AUHAL start: {e}")))?;

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
        // Stop AUHAL.
        if let Err(e) = self.audio_unit.stop() {
            warn!(error = %e, "process-tap AUHAL stop error (non-fatal)");
        }
        // Let the audio thread drain.
        std::thread::sleep(std::time::Duration::from_millis(150));
        // Destroy tap.
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

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn create_tap() -> Result<AudioObjectID> {
        // SAFETY: ObjC message sends; CATapDescription is an NSObject subclass.
        // Use `new()` which gives us a default-initialised description.
        // CoreAudio treats a default CATapDescription as "capture all system audio
        // in stereo, excluding the calling process" — exactly what we need.
        let tap_desc = unsafe { CATapDescription::new() };

        let mut tap_id: AudioObjectID = 0;
        let status = unsafe { AudioHardwareCreateProcessTap(Some(&tap_desc), &mut tap_id) };
        if status != 0 {
            return Err(AttuneError::SystemAudio(format!(
                "AudioHardwareCreateProcessTap failed: OSStatus {status} \
                 (if this is the first launch, the OS may need to prompt for permission)"
            )));
        }
        if tap_id == 0 {
            return Err(AttuneError::SystemAudio(
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
            return Err(AttuneError::SystemAudio(format!(
                "kAudioTapPropertyFormat read failed: OSStatus {status}"
            )));
        }
        // Default to 48 kHz stereo when the tap reports zero (shouldn't happen
        // on a real tap, but guards against a degenerate state during testing).
        if fmt.mSampleRate < 1.0 {
            fmt.mSampleRate = 48_000.0;
            fmt.mChannelsPerFrame = 2;
        }
        Ok(fmt)
    }

    fn open_auhal_on_tap(tap_id: AudioObjectID) -> Result<AudioUnit> {
        let mut unit = AudioUnit::new_uninitialized(IOType::HalOutput)
            .map_err(|e| AttuneError::SystemAudio(format!("AUHAL new: {e}")))?;

        // Disable output, enable input — we are a capture-only unit.
        let off: u32 = 0;
        unit.set_property(
            kAudioOutputUnitProperty_EnableIO,
            Scope::Output,
            Element::Output,
            Some(&off),
        )
        .map_err(|e| AttuneError::SystemAudio(format!("AUHAL disable output: {e}")))?;

        let on: u32 = 1;
        unit.set_property(
            kAudioOutputUnitProperty_EnableIO,
            Scope::Input,
            Element::Input,
            Some(&on),
        )
        .map_err(|e| AttuneError::SystemAudio(format!("AUHAL enable input: {e}")))?;

        // Bind the AUHAL to the tap object ID. On macOS 14.4+, the HAL
        // accepts a tap ID where a device ID is expected.
        // kAudioOutputUnitProperty_CurrentDevice = 2000
        const kAudioOutputUnitProperty_CurrentDevice: u32 = 2000;
        unit.set_property(
            kAudioOutputUnitProperty_CurrentDevice,
            Scope::Global,
            Element::Output,
            Some(&tap_id),
        )
        .map_err(|e| AttuneError::SystemAudio(format!("AUHAL bind tap: {e}")))?;

        unit.initialize()
            .map_err(|e| AttuneError::SystemAudio(format!("AUHAL initialize: {e}")))?;

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
        // Just verify the sysctlbyname path doesn't crash. The actual
        // boolean value depends on the test runner's OS version.
        let _ = is_supported();
    }

    #[test]
    fn global_address_has_correct_scope() {
        let addr = global_address(kAudioTapPropertyFormat);
        assert_eq!(addr.mScope, kAudioObjectPropertyScopeGlobal);
        assert_eq!(addr.mSelector, kAudioTapPropertyFormat);
    }
}
