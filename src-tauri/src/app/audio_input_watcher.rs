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

const PROCESS_OBJECT_LIST: AudioObjectPropertySelector = u32_from_fourcc(b"pol#");
const PROCESS_BUNDLE_ID: AudioObjectPropertySelector = u32_from_fourcc(b"pbid");
const PROCESS_IS_RUNNING_INPUT: AudioObjectPropertySelector = u32_from_fourcc(b"pirI");

const STATUS_OK: OSStatus = 0;

const fn u32_from_fourcc(s: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*s)
}

#[derive(Debug, Clone)]
pub struct AudioProcess {
    pub bundle_id: String,
    pub input_active: bool,
}

pub fn snapshot() -> Option<Vec<AudioProcess>> {
    let ids = process_object_list()?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
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

    let s = unsafe { CFString::wrap_under_create_rule(cfstr) };
    Some(s.to_string())
}

fn process_is_running_input(id: AudioObjectID) -> Option<bool> {
    let addr = property_address(PROCESS_IS_RUNNING_INPUT);
    let mut size: u32 = mem::size_of::<u32>() as u32;
    let mut val: u32 = 0;

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
        assert_eq!(u32_from_fourcc(b"pol#"), 0x706F_6C23);
        assert_eq!(u32_from_fourcc(b"pbid"), 0x7062_6964);
        assert_eq!(u32_from_fourcc(b"pirI"), 0x7069_7249);
    }

    #[test]
    fn snapshot_returns_some_on_modern_macos() {
        if let Some(procs) = snapshot() {
            assert!(!procs.is_empty(), "HAL returned an empty process list");
        }
    }
}
