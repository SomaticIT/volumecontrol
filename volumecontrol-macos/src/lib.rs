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
    #[allow(dead_code)]
    id: String,
    /// Human-readable device name (`kAudioObjectPropertyName`).
    #[allow(dead_code)]
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
    fn default() -> Result<Self, AudioError> {
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
            // Partial, case-sensitive substring match: returns the first device
            // whose name contains `name`.  This mirrors the behaviour of the
            // other platform backends and gives callers flexibility (e.g. "Air"
            // matches "AirPods Pro").
            for raw_id in internal::list_device_ids()? {
                let device_name = internal::get_device_name(raw_id)?;
                if device_name.contains(name) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use volumecontrol_core::AudioDevice as AudioDeviceTrait;

    #[test]
    fn default_returns_unsupported_without_feature() {
        let result = AudioDevice::default();
        assert!(result.is_err());
        #[cfg(not(feature = "coreaudio"))]
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[test]
    fn from_id_returns_unsupported_without_feature() {
        let result = AudioDevice::from_id("test-id");
        assert!(result.is_err());
        #[cfg(not(feature = "coreaudio"))]
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[test]
    fn from_name_returns_unsupported_without_feature() {
        let result = AudioDevice::from_name("test-name");
        assert!(result.is_err());
        #[cfg(not(feature = "coreaudio"))]
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[test]
    fn list_returns_unsupported_without_feature() {
        let result = AudioDevice::list();
        assert!(result.is_err());
        #[cfg(not(feature = "coreaudio"))]
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }
}
