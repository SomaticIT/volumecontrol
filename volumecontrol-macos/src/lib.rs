//! macOS CoreAudio volume control backend.
//!
//! This crate exposes an [`AudioDevice`] type that implements
//! [`volumecontrol_core::AudioDevice`].  When the `coreaudio` feature is
//! **not** enabled every method returns [`AudioError::Unsupported`], which
//! allows the crate to be compiled on any platform without the CoreAudio SDK.
//!
//! When the `coreaudio` feature **is** enabled the implementation bridges to
//! the native macOS CoreAudio Hardware Abstraction Layer (HAL) via the
//! [`objc2_core_audio`] bindings.  All unsafe interactions with CoreAudio are
//! contained in the [`internal`] module.

mod internal;

use volumecontrol_core::{AudioDevice as AudioDeviceTrait, AudioError};

/// Represents a CoreAudio audio output device (macOS).
///
/// # Feature flags
///
/// Real CoreAudio integration requires the `coreaudio` feature and must be
/// built for a macOS target.  Without the feature every method returns
/// [`AudioError::Unsupported`].
#[derive(Debug)]
pub struct AudioDevice {
    /// CoreAudio `AudioObjectID` (serialized as a string for the public API).
    id: String,
    /// Human-readable device name (`kAudioObjectPropertyName`).
    name: String,
}

#[cfg(feature = "coreaudio")]
impl AudioDevice {
    /// Constructs an [`AudioDevice`] from a raw CoreAudio `AudioObjectID`.
    fn from_raw_id(raw_id: internal::AudioObjectID) -> Result<Self, AudioError> {
        let name = internal::get_device_name(raw_id)?;
        Ok(Self {
            id: raw_id.to_string(),
            name,
        })
    }
}

impl AudioDeviceTrait for AudioDevice {
    fn from_default() -> Result<Self, AudioError> {
        #[cfg(feature = "coreaudio")]
        {
            let raw_id = internal::get_default_device_id()?;
            Self::from_raw_id(raw_id)
        }
        #[cfg(not(feature = "coreaudio"))]
        Err(AudioError::Unsupported)
    }

    fn from_id(id: &str) -> Result<Self, AudioError> {
        #[cfg(feature = "coreaudio")]
        {
            // The public `id` is the decimal string representation of the
            // `AudioObjectID`.  Parse it back and verify the device exists by
            // fetching its name.
            let raw_id: internal::AudioObjectID =
                id.parse().map_err(|_| AudioError::DeviceNotFound)?;
            // Listing devices lets us confirm this ID is valid.
            let ids = internal::list_device_ids()?;
            if !ids.contains(&raw_id) {
                return Err(AudioError::DeviceNotFound);
            }
            Self::from_raw_id(raw_id)
        }
        #[cfg(not(feature = "coreaudio"))]
        {
            let _ = id;
            Err(AudioError::Unsupported)
        }
    }

    fn from_name(name: &str) -> Result<Self, AudioError> {
        #[cfg(feature = "coreaudio")]
        {
            // Case-insensitive substring match: returns the first device
            // whose name contains `name`.  This gives callers flexibility
            // (e.g. "airpods" matches "AirPods Pro").
            let name_lower = name.to_lowercase();
            for raw_id in internal::list_device_ids()? {
                let device_name = internal::get_device_name(raw_id)?;
                if device_name.to_lowercase().contains(&name_lower) {
                    return Self::from_raw_id(raw_id);
                }
            }
            Err(AudioError::DeviceNotFound)
        }
        #[cfg(not(feature = "coreaudio"))]
        {
            let _ = name;
            Err(AudioError::Unsupported)
        }
    }

    fn list() -> Result<Vec<(String, String)>, AudioError> {
        #[cfg(feature = "coreaudio")]
        {
            let ids = internal::list_device_ids()?;
            let mut devices = Vec::with_capacity(ids.len());
            for raw_id in ids {
                let name = internal::get_device_name(raw_id)?;
                devices.push((raw_id.to_string(), name));
            }
            Ok(devices)
        }
        #[cfg(not(feature = "coreaudio"))]
        Err(AudioError::Unsupported)
    }

    fn get_vol(&self) -> Result<u8, AudioError> {
        #[cfg(feature = "coreaudio")]
        {
            let raw_id: internal::AudioObjectID =
                self.id.parse().map_err(|_| AudioError::DeviceNotFound)?;
            internal::get_volume(raw_id)
        }
        #[cfg(not(feature = "coreaudio"))]
        Err(AudioError::Unsupported)
    }

    fn set_vol(&self, vol: u8) -> Result<(), AudioError> {
        #[cfg(feature = "coreaudio")]
        {
            let raw_id: internal::AudioObjectID =
                self.id.parse().map_err(|_| AudioError::DeviceNotFound)?;
            internal::set_volume(raw_id, vol)
        }
        #[cfg(not(feature = "coreaudio"))]
        {
            let _ = vol;
            Err(AudioError::Unsupported)
        }
    }

    fn is_mute(&self) -> Result<bool, AudioError> {
        #[cfg(feature = "coreaudio")]
        {
            let raw_id: internal::AudioObjectID =
                self.id.parse().map_err(|_| AudioError::DeviceNotFound)?;
            internal::get_mute(raw_id)
        }
        #[cfg(not(feature = "coreaudio"))]
        Err(AudioError::Unsupported)
    }

    fn set_mute(&self, muted: bool) -> Result<(), AudioError> {
        #[cfg(feature = "coreaudio")]
        {
            let raw_id: internal::AudioObjectID =
                self.id.parse().map_err(|_| AudioError::DeviceNotFound)?;
            internal::set_mute(raw_id, muted)
        }
        #[cfg(not(feature = "coreaudio"))]
        {
            let _ = muted;
            Err(AudioError::Unsupported)
        }
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use volumecontrol_core::AudioDevice as AudioDeviceTrait;

    // ── stub tests (no coreaudio feature) ────────────────────────────────────
    // These tests are only compiled and run when the `coreaudio` feature is
    // disabled; with the feature enabled the methods do real work instead of
    // returning `Unsupported`.

    #[cfg(not(feature = "coreaudio"))]
    #[test]
    fn default_returns_unsupported_without_feature() {
        let result = AudioDevice::from_default();
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[cfg(not(feature = "coreaudio"))]
    #[test]
    fn from_id_returns_unsupported_without_feature() {
        let result = AudioDevice::from_id("test-id");
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[cfg(not(feature = "coreaudio"))]
    #[test]
    fn from_name_returns_unsupported_without_feature() {
        let result = AudioDevice::from_name("test-name");
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[cfg(not(feature = "coreaudio"))]
    #[test]
    fn list_returns_unsupported_without_feature() {
        let result = AudioDevice::list();
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    // ── real-world tests (coreaudio feature, macOS only) ─────────────────────
    // These tests exercise the actual CoreAudio stack and therefore only run on
    // macOS with a real audio hardware HAL available.

    #[cfg(all(feature = "coreaudio", target_os = "macos"))]
    #[test]
    fn default_returns_ok() {
        let device = AudioDevice::from_default();
        assert!(device.is_ok(), "expected Ok, got {device:?}");
    }

    #[cfg(all(feature = "coreaudio", target_os = "macos"))]
    #[test]
    fn list_returns_nonempty() {
        let devices = AudioDevice::list().expect("list()");
        assert!(
            !devices.is_empty(),
            "expected at least one audio device from list()"
        );
        // Every entry must have a non-empty id and name.
        for (id, name) in &devices {
            assert!(!id.is_empty(), "device id must not be empty");
            assert!(!name.is_empty(), "device name must not be empty");
        }
    }

    #[cfg(all(feature = "coreaudio", target_os = "macos"))]
    #[test]
    fn from_id_valid_id_returns_ok() {
        // Use the default device's id to look up via `from_id`.
        let default_device = AudioDevice::from_default().expect("from_default()");
        let found = AudioDevice::from_id(default_device.id());
        assert!(found.is_ok(), "from_id with valid id should succeed");
        assert_eq!(found.unwrap().id(), default_device.id());
    }

    #[cfg(all(feature = "coreaudio", target_os = "macos"))]
    #[test]
    fn from_id_invalid_id_returns_not_found() {
        let result = AudioDevice::from_id("not-a-number");
        assert!(
            matches!(result.unwrap_err(), AudioError::DeviceNotFound),
            "non-numeric id should return DeviceNotFound"
        );
    }

    #[cfg(all(feature = "coreaudio", target_os = "macos"))]
    #[test]
    fn from_name_partial_match_returns_ok() {
        // Build a partial name from the first few characters of the default
        // device name to guarantee a match without hard-coding a device name.
        let default_device = AudioDevice::from_default().expect("from_default()");
        let partial: String = default_device.name().chars().take(3).collect();
        let found = AudioDevice::from_name(&partial);
        assert!(
            found.is_ok(),
            "from_name with partial match '{partial}' should succeed"
        );
    }

    #[cfg(all(feature = "coreaudio", target_os = "macos"))]
    #[test]
    fn from_name_case_insensitive_match_returns_ok() {
        // Convert the default device name to uppercase and verify it still
        // matches — confirming that `from_name` is case-insensitive.
        let default_device = AudioDevice::from_default().expect("from_default()");
        let upper = default_device.name().to_uppercase();
        let found = AudioDevice::from_name(&upper);
        assert!(
            found.is_ok(),
            "from_name with uppercase query '{upper}' should succeed (case-insensitive)"
        );
    }

    #[cfg(all(feature = "coreaudio", target_os = "macos"))]
    #[test]
    fn from_name_no_match_returns_not_found() {
        let result = AudioDevice::from_name("\x00\x01\x02");
        assert!(
            matches!(result.unwrap_err(), AudioError::DeviceNotFound),
            "unrecognised name should return DeviceNotFound"
        );
    }

    #[cfg(all(feature = "coreaudio", target_os = "macos"))]
    #[test]
    fn get_vol_returns_valid_range() {
        let device = AudioDevice::from_default().expect("from_default()");
        let vol = device.get_vol().expect("get_vol()");
        assert!(vol <= 100, "volume must be in 0..=100, got {vol}");
    }

    #[cfg(all(feature = "coreaudio", target_os = "macos"))]
    #[test]
    fn set_vol_roundtrip() {
        let device = AudioDevice::from_default().expect("from_default()");
        let original = device.get_vol().expect("get_vol()");
        device.set_vol(original).expect("set_vol()");
        let after = device.get_vol().expect("get_vol() after set");
        // Allow ±1 rounding error due to f32 ↔ u8 conversion.
        assert!(
            original.abs_diff(after) <= 1,
            "volume changed unexpectedly: {original} -> {after}"
        );
    }

    #[cfg(all(feature = "coreaudio", target_os = "macos"))]
    #[test]
    fn set_mute_roundtrip() {
        let device = AudioDevice::from_default().expect("from_default()");
        let original = device.is_mute().expect("is_mute()");
        device.set_mute(original).expect("set_mute()");
        let after = device.is_mute().expect("is_mute() after set");
        assert_eq!(original, after, "mute state changed unexpectedly");
    }
}
