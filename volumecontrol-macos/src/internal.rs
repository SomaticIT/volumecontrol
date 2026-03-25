//! Internal low-level CoreAudio utilities.
//!
//! This module encapsulates all unsafe interactions with the CoreAudio C API
//! so that the public-facing [`crate::AudioDevice`] implementation remains
//! safe.  Every function here is `pub(crate)` and is compiled only when the
//! `coreaudio` feature is enabled.

#![cfg(feature = "coreaudio")]

use std::ffi::c_void;
use std::mem;
use std::ptr::NonNull;

use objc2_core_audio::{
    kAudioDevicePropertyMute, kAudioDevicePropertyVolumeScalar, kAudioHardwareNoError,
    kAudioHardwarePropertyDefaultOutputDevice, kAudioHardwarePropertyDevices,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyName, kAudioObjectPropertyScopeGlobal,
    kAudioObjectPropertyScopeOutput, kAudioObjectSystemObject, AudioObjectGetPropertyData,
    AudioObjectGetPropertyDataSize, AudioObjectPropertyAddress, AudioObjectSetPropertyData,
};
use objc2_core_foundation::{CFRetained, CFString};

use volumecontrol_core::AudioError;

/// CoreAudio `AudioObjectID` type alias (always `u32`).
pub(crate) type AudioObjectID = objc2_core_audio::AudioObjectID;

// ── helpers ─────────────────────────────────────────────────────────────────

/// Queries the byte count of a CoreAudio property.
///
/// # Safety
///
/// `object_id` must be a valid `AudioObjectID` and `address` must describe a
/// property that exists on that object.
unsafe fn get_property_data_size(
    object_id: AudioObjectID,
    address: &mut AudioObjectPropertyAddress,
) -> Result<u32, AudioError> {
    let mut size: u32 = 0;
    // SAFETY: address is a valid local struct; size is properly initialized.
    let status = unsafe {
        AudioObjectGetPropertyDataSize(
            object_id,
            NonNull::from(address),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
        )
    };
    if status != kAudioHardwareNoError {
        return Err(AudioError::ListFailed(format!(
            "AudioObjectGetPropertyDataSize failed with status {status}"
        )));
    }
    Ok(size)
}

// ── public(crate) API ────────────────────────────────────────────────────────

/// Returns the `AudioObjectID` of the system default audio output device.
///
/// # Errors
///
/// Returns [`AudioError::InitializationFailed`] if the CoreAudio call fails,
/// or [`AudioError::DeviceNotFound`] if no default device is available.
pub(crate) fn get_default_device_id() -> Result<AudioObjectID, AudioError> {
    let mut address = AudioObjectPropertyAddress {
        mSelector: kAudioHardwarePropertyDefaultOutputDevice,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut device_id: AudioObjectID = 0;
    let mut data_size = mem::size_of::<AudioObjectID>() as u32;

    // SAFETY: address is a valid local struct; device_id and data_size are
    // properly initialised to the correct sizes.
    let status = unsafe {
        AudioObjectGetPropertyData(
            kAudioObjectSystemObject as AudioObjectID,
            NonNull::from(&mut address),
            0,
            std::ptr::null::<c_void>(),
            NonNull::from(&mut data_size),
            NonNull::new((&raw mut device_id).cast::<c_void>()).unwrap(),
        )
    };

    if status != kAudioHardwareNoError {
        return Err(AudioError::InitializationFailed(format!(
            "AudioObjectGetPropertyData for default output device failed with status {status}"
        )));
    }
    if device_id == 0 {
        return Err(AudioError::DeviceNotFound);
    }
    Ok(device_id)
}

/// Lists all `AudioObjectID`s registered as audio devices with CoreAudio.
///
/// # Errors
///
/// Returns [`AudioError::ListFailed`] if the CoreAudio calls fail.
pub(crate) fn list_device_ids() -> Result<Vec<AudioObjectID>, AudioError> {
    let mut address = AudioObjectPropertyAddress {
        mSelector: kAudioHardwarePropertyDevices,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };

    // SAFETY: address is a valid local struct; the helper only reads it.
    let byte_count =
        unsafe { get_property_data_size(kAudioObjectSystemObject as AudioObjectID, &mut address)? };

    let count = byte_count as usize / mem::size_of::<AudioObjectID>();
    let mut ids: Vec<AudioObjectID> = vec![0u32; count];
    let mut data_size = byte_count;

    // SAFETY: address is valid; ids is allocated to the required size;
    // data_size reflects the allocated buffer.
    let status = unsafe {
        AudioObjectGetPropertyData(
            kAudioObjectSystemObject as AudioObjectID,
            NonNull::from(&mut address),
            0,
            std::ptr::null::<c_void>(),
            NonNull::from(&mut data_size),
            NonNull::new(ids.as_mut_ptr().cast::<c_void>()).unwrap(),
        )
    };

    if status != kAudioHardwareNoError {
        return Err(AudioError::ListFailed(format!(
            "AudioObjectGetPropertyData for device list failed with status {status}"
        )));
    }
    Ok(ids)
}

/// Returns the human-readable name of the audio device identified by `id`.
///
/// CoreAudio returns the name as a retained `CFString`; this function
/// converts it into an owned [`String`] and releases the CoreFoundation
/// object.
///
/// # Errors
///
/// Returns [`AudioError::ListFailed`] if the name cannot be retrieved.
pub(crate) fn get_device_name(id: AudioObjectID) -> Result<String, AudioError> {
    let mut address = AudioObjectPropertyAddress {
        mSelector: kAudioObjectPropertyName,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };

    // `kAudioObjectPropertyName` returns a *retained* CFString.
    // We store the raw pointer and wrap it in `CFRetained` to ensure it is
    // released when this function returns.
    let mut cf_str_ptr: *mut CFString = std::ptr::null_mut();
    let mut data_size = mem::size_of::<*mut CFString>() as u32;

    // SAFETY: address is a valid local struct; cf_str_ptr is a valid pointer
    // location for a CFStringRef; data_size is correct.
    let status = unsafe {
        AudioObjectGetPropertyData(
            id,
            NonNull::from(&mut address),
            0,
            std::ptr::null::<c_void>(),
            NonNull::from(&mut data_size),
            NonNull::new((&raw mut cf_str_ptr).cast::<c_void>()).unwrap(),
        )
    };

    if status != kAudioHardwareNoError {
        return Err(AudioError::ListFailed(format!(
            "AudioObjectGetPropertyData for device name failed with status {status}"
        )));
    }

    let name = match NonNull::new(cf_str_ptr) {
        None => {
            return Err(AudioError::ListFailed(
                "device name CFString was null".into(),
            ))
        }
        Some(ptr) => {
            // SAFETY: CoreAudio returns a +1 retained CFString here (Create
            // Rule); wrapping it in CFRetained takes ownership and will call
            // CFRelease when dropped.
            let retained: CFRetained<CFString> = unsafe { CFRetained::from_raw(ptr) };
            format!("{retained}")
        }
    };

    Ok(name)
}

/// Returns the scalar volume (0 – 100) of the output device `id`.
///
/// Reads `kAudioDevicePropertyVolumeScalar` on element 0 (master channel)
/// of the output scope.
///
/// # Errors
///
/// Returns [`AudioError::GetVolumeFailed`] if the CoreAudio call fails.
pub(crate) fn get_volume(id: AudioObjectID) -> Result<u8, AudioError> {
    let mut address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyVolumeScalar,
        mScope: kAudioObjectPropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut scalar: f32 = 0.0;
    let mut data_size = mem::size_of::<f32>() as u32;

    // SAFETY: address, scalar, and data_size are all valid local values of
    // the correct sizes for this property.
    let status = unsafe {
        AudioObjectGetPropertyData(
            id,
            NonNull::from(&mut address),
            0,
            std::ptr::null::<c_void>(),
            NonNull::from(&mut data_size),
            NonNull::new((&raw mut scalar).cast::<c_void>()).unwrap(),
        )
    };

    if status != kAudioHardwareNoError {
        return Err(AudioError::GetVolumeFailed(format!(
            "AudioObjectGetPropertyData for volume failed with status {status}"
        )));
    }

    // Clamp to [0.0, 1.0] before conversion in case CoreAudio returns a
    // slightly out-of-range value.
    let clamped = scalar.clamp(0.0_f32, 1.0_f32);
    Ok((clamped * 100.0_f32).round() as u8)
}

/// Sets the scalar volume of the output device `id`.
///
/// `vol` is expected to be in `0..=100` and is divided by 100 before being
/// written to CoreAudio.
///
/// # Errors
///
/// Returns [`AudioError::SetVolumeFailed`] if the CoreAudio call fails.
pub(crate) fn set_volume(id: AudioObjectID, vol: u8) -> Result<(), AudioError> {
    let mut address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyVolumeScalar,
        mScope: kAudioObjectPropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain,
    };
    let scalar: f32 = f32::from(vol.min(100)) / 100.0_f32;
    let data_size = mem::size_of::<f32>() as u32;

    // SAFETY: address, scalar, and data_size are all valid local values of
    // the correct sizes for this property.  `AudioObjectSetPropertyData` only
    // reads from `in_data`, so casting the const pointer to mut is sound.
    let status = unsafe {
        AudioObjectSetPropertyData(
            id,
            NonNull::from(&mut address),
            0,
            std::ptr::null::<c_void>(),
            data_size,
            NonNull::new(std::ptr::addr_of!(scalar).cast_mut().cast::<c_void>()).unwrap(),
        )
    };

    if status != kAudioHardwareNoError {
        return Err(AudioError::SetVolumeFailed(format!(
            "AudioObjectSetPropertyData for volume failed with status {status}"
        )));
    }
    Ok(())
}

/// Returns `true` if the output device `id` is muted.
///
/// Reads `kAudioDevicePropertyMute` on element 0 (master channel) of the
/// output scope.
///
/// # Errors
///
/// Returns [`AudioError::GetMuteFailed`] if the CoreAudio call fails.
pub(crate) fn get_mute(id: AudioObjectID) -> Result<bool, AudioError> {
    let mut address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyMute,
        mScope: kAudioObjectPropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut mute: u32 = 0;
    let mut data_size = mem::size_of::<u32>() as u32;

    // SAFETY: address, mute, and data_size are all valid local values of the
    // correct sizes for this property.
    let status = unsafe {
        AudioObjectGetPropertyData(
            id,
            NonNull::from(&mut address),
            0,
            std::ptr::null::<c_void>(),
            NonNull::from(&mut data_size),
            NonNull::new((&raw mut mute).cast::<c_void>()).unwrap(),
        )
    };

    if status != kAudioHardwareNoError {
        return Err(AudioError::GetMuteFailed(format!(
            "AudioObjectGetPropertyData for mute failed with status {status}"
        )));
    }
    Ok(mute != 0)
}

/// Mutes or unmutes the output device `id`.
///
/// # Errors
///
/// Returns [`AudioError::SetMuteFailed`] if the CoreAudio call fails.
pub(crate) fn set_mute(id: AudioObjectID, muted: bool) -> Result<(), AudioError> {
    let mut address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyMute,
        mScope: kAudioObjectPropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mute: u32 = u32::from(muted);
    let data_size = mem::size_of::<u32>() as u32;

    // SAFETY: address, mute, and data_size are all valid local values of the
    // correct sizes for this property.  `AudioObjectSetPropertyData` only
    // reads from `in_data`, so casting the const pointer to mut is sound.
    let status = unsafe {
        AudioObjectSetPropertyData(
            id,
            NonNull::from(&mut address),
            0,
            std::ptr::null::<c_void>(),
            data_size,
            NonNull::new(std::ptr::addr_of!(mute).cast_mut().cast::<c_void>()).unwrap(),
        )
    };

    if status != kAudioHardwareNoError {
        return Err(AudioError::SetMuteFailed(format!(
            "AudioObjectSetPropertyData for mute failed with status {status}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    // Tests for the internal module run only on macOS with the `coreaudio`
    // feature enabled.  Without the feature the module is not compiled at all,
    // so non-macOS CI jobs simply skip these tests.
    //
    // The `#[cfg(target_os = "macos")]` guard ensures the tests are only
    // executed when a real CoreAudio stack is present.

    #[cfg(target_os = "macos")]
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn default_device_id_is_nonzero() {
        let id = get_default_device_id();
        assert!(id.is_ok(), "expected Ok, got {id:?}");
        assert_ne!(id.unwrap(), 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn device_list_is_nonempty() {
        let ids = list_device_ids();
        assert!(ids.is_ok(), "expected Ok, got {ids:?}");
        assert!(
            !ids.unwrap().is_empty(),
            "expected at least one audio device"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn default_device_has_name() {
        let id = get_default_device_id().expect("default device");
        let name = get_device_name(id);
        assert!(name.is_ok(), "expected Ok, got {name:?}");
        assert!(!name.unwrap().is_empty(), "device name should not be empty");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn get_and_set_volume_roundtrip() {
        let id = get_default_device_id().expect("default device");
        let original = get_volume(id).expect("get_volume");
        set_volume(id, original).expect("set_volume");
        let after = get_volume(id).expect("get_volume after set");
        // Allow ±1 rounding error due to f32 ↔ u8 conversion.
        assert!(
            original.abs_diff(after) <= 1,
            "volume changed: {original} -> {after}"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn get_and_set_mute_roundtrip() {
        let id = get_default_device_id().expect("default device");
        let original = get_mute(id).expect("get_mute");
        set_mute(id, original).expect("set_mute");
        let after = get_mute(id).expect("get_mute after set");
        assert_eq!(original, after);
    }
}
