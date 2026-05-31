//! Audio HAL per-process input watcher (macOS 14.2+).
//!
//! macOS 14.2 added a per-process view of the audio HAL: each process the
//! audio server is aware of gets an `AudioObjectID` and exposes its PID,
//! bundle id, and an `IsRunningInput` boolean — the same signal that
//! powers the orange microphone dot in the menu bar. By polling this we
//! get "the user actually opened a mic stream", which fires the moment
//! someone joins a Discord voice channel, a Zoom call, a Teams meeting,
//! or a FaceTime — regardless of when the app itself was launched. That
//! is the signal Granola watches, and it is the right one for us too:
//! the existing `NSWorkspace.runningApplications` edge only fires on
//! cold launch, so apps like Discord that boot at login never trigger.
//!
//! [`snapshot`] returns `None` when the new property selectors are not
//! recognised by the running audio server (older OS, or transient
//! startup race). The caller is expected to degrade to a process-list
//! fallback when that happens.

use std::ffi::c_void;
use std::mem;
use std::ptr;

use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};
use coreaudio_sys::{
    kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal, kAudioObjectSystemObject,
    AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize, AudioObjectID,
    AudioObjectPropertyAddress, AudioObjectPropertySelector, OSStatus,
};

/// macOS 14.2+ Audio HAL selectors. Hardcoded as big-endian FourCC u32s
/// so this crate compiles against any SDK — what matters is the running
/// `coreaudiod` recognising them, and if it does not we degrade.
const PROCESS_OBJECT_LIST: AudioObjectPropertySelector = u32_from_fourcc(b"pol#");
const PROCESS_BUNDLE_ID: AudioObjectPropertySelector = u32_from_fourcc(b"pbid");
const PROCESS_IS_RUNNING_INPUT: AudioObjectPropertySelector = u32_from_fourcc(b"pirI");

const STATUS_OK: OSStatus = 0;

/// Compile-time FourCC → big-endian u32. CoreAudio uses these as
/// `AudioObjectPropertySelector` values; `'pol#'` etc. in the headers.
const fn u32_from_fourcc(s: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*s)
}

/// One process the audio HAL has registered.
#[derive(Debug, Clone)]
pub struct AudioProcess {
    pub bundle_id: String,
    pub input_active: bool,
}

/// Snapshot every audio process the HAL knows about. `None` when the
/// macOS 14.2+ properties are not available; callers should treat that
/// as "fall back to the process-list watcher".
pub fn snapshot() -> Option<Vec<AudioProcess>> {
    let ids = process_object_list()?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        // Helper daemons and background services have no bundle id; only
        // user-facing apps do. Skip the rest — they cannot be meetings.
        let Some(bundle_id) = process_bundle_id(id) else {
            continue;
        };
        let input_active = process_is_running_input(id).unwrap_or(false);
        out.push(AudioProcess {
            bundle_id,
            input_active,
        });
    }
    Some(out)
}

fn property_address(selector: AudioObjectPropertySelector) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    }
}

fn process_object_list() -> Option<Vec<AudioObjectID>> {
    let addr = property_address(PROCESS_OBJECT_LIST);

    let mut size: u32 = 0;
    // SAFETY: `kAudioObjectSystemObject` is the documented root id and
    // `addr` is a borrowed pointer to a stack-local struct that outlives
    // the call. The call only reads through both, never stores them.
    let status = unsafe {
        AudioObjectGetPropertyDataSize(
            kAudioObjectSystemObject,
            &addr as *const _,
            0,
            ptr::null(),
            &mut size,
        )
    };
    if status != STATUS_OK || size == 0 {
        return None;
    }

    let count = size as usize / mem::size_of::<AudioObjectID>();
    let mut ids: Vec<AudioObjectID> = vec![0; count];
    let mut io_size = size;
    // SAFETY: `ids` is sized to `size` bytes (count × sizeof(AudioObjectID)),
    // matches what the prior size query reported, and we pass a mutable
    // pointer plus the same property address.
    let status = unsafe {
        AudioObjectGetPropertyData(
            kAudioObjectSystemObject,
            &addr as *const _,
            0,
            ptr::null(),
            &mut io_size,
            ids.as_mut_ptr() as *mut c_void,
        )
    };
    if status != STATUS_OK {
        return None;
    }
    ids.truncate(io_size as usize / mem::size_of::<AudioObjectID>());
    Some(ids)
}

fn process_bundle_id(id: AudioObjectID) -> Option<String> {
    let addr = property_address(PROCESS_BUNDLE_ID);
    let mut size: u32 = mem::size_of::<CFStringRef>() as u32;
    let mut cfstr: CFStringRef = ptr::null();
    // SAFETY: `cfstr` is a stack-local CFStringRef slot; the call writes
    // a +1-retained CFString pointer into it (the standard "Create Rule"
    // CoreAudio uses for object properties of CFType).
    let status = unsafe {
        AudioObjectGetPropertyData(
            id,
            &addr as *const _,
            0,
            ptr::null(),
            &mut size,
            &mut cfstr as *mut CFStringRef as *mut c_void,
        )
    };
    if status != STATUS_OK || cfstr.is_null() {
        return None;
    }
    // SAFETY: cfstr is +1-retained (Create Rule). `wrap_under_create_rule`
    // takes ownership of that retain and releases on drop.
    let s = unsafe { CFString::wrap_under_create_rule(cfstr) };
    Some(s.to_string())
}

fn process_is_running_input(id: AudioObjectID) -> Option<bool> {
    let addr = property_address(PROCESS_IS_RUNNING_INPUT);
    let mut size: u32 = mem::size_of::<u32>() as u32;
    let mut val: u32 = 0;
    // SAFETY: writes a single u32 (the AudioHardware boolean
    // representation) into a stack slot of matching size.
    let status = unsafe {
        AudioObjectGetPropertyData(
            id,
            &addr as *const _,
            0,
            ptr::null(),
            &mut size,
            &mut val as *mut u32 as *mut c_void,
        )
    };
    if status != STATUS_OK {
        return None;
    }
    Some(val != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fourcc_encodes_in_big_endian_order() {
        // The HAL selectors must be readable as 'pol#', 'pbid', 'pirI'
        // from a hex dump — i.e. the first byte of the literal is the
        // most significant byte of the resulting u32.
        assert_eq!(u32_from_fourcc(b"pol#"), 0x706F_6C23);
        assert_eq!(u32_from_fourcc(b"pbid"), 0x7062_6964);
        assert_eq!(u32_from_fourcc(b"pirI"), 0x7069_7249);
    }

    #[test]
    fn snapshot_returns_some_on_modern_macos() {
        // Smoke test: on a developer machine running macOS 14.2+ we
        // should always get *something* back (every Mac has at least
        // `coreaudiod` and `WindowServer` audio-aware processes). On
        // older OS or in CI without an audio server this will be None,
        // which is the expected fallback signal.
        if let Some(procs) = snapshot() {
            assert!(!procs.is_empty(), "HAL returned an empty process list");
        }
    }
}
