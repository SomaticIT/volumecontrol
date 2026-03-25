mod internal;

use volumecontrol_core::{AudioDevice as AudioDeviceTrait, AudioError};

/// Represents a WASAPI audio output device (Windows).
///
/// # Feature flags
///
/// Real WASAPI integration requires the `wasapi` feature and must be built
/// for a Windows target.  Without the feature every method returns
/// [`AudioError::Unsupported`].
#[derive(Debug)]
pub struct AudioDevice {
    /// WASAPI endpoint identifier (GUID string).
    // Only accessed via the `wasapi` feature path; suppress dead_code on
    // non-Windows builds.
    #[allow(dead_code)]
    id: String,
    /// Friendly device name.
    #[allow(dead_code)]
    name: String,
}

impl AudioDeviceTrait for AudioDevice {
    /// Returns the system default audio render device.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::InitializationFailed`] if COM cannot be
    /// initialised or if the default device cannot be resolved.
    /// Returns [`AudioError::Unsupported`] when the `wasapi` feature is
    /// not enabled.
    fn default() -> Result<Self, AudioError> {
        #[cfg(feature = "wasapi")]
        {
            let _com = internal::wasapi::ComGuard::new()?;
            let enumerator = internal::wasapi::create_enumerator()?;
            let device = internal::wasapi::get_default_device(&enumerator)?;
            let id = internal::wasapi::device_id(&device)?;
            let name = internal::wasapi::device_name(&device)?;
            Ok(Self { id, name })
        }
        #[cfg(not(feature = "wasapi"))]
        Err(AudioError::Unsupported)
    }

    /// Returns the audio device identified by `id`.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::DeviceNotFound`] if no device with the given
    /// identifier exists.
    /// Returns [`AudioError::InitializationFailed`] if COM cannot be
    /// initialised or another lookup failure occurs.
    /// Returns [`AudioError::Unsupported`] when the `wasapi` feature is
    /// not enabled.
    fn from_id(id: &str) -> Result<Self, AudioError> {
        #[cfg(feature = "wasapi")]
        {
            let _com = internal::wasapi::ComGuard::new()?;
            let enumerator = internal::wasapi::create_enumerator()?;
            let device = internal::wasapi::get_device_by_id(&enumerator, id)?;
            let resolved_id = internal::wasapi::device_id(&device)?;
            let name = internal::wasapi::device_name(&device)?;
            Ok(Self {
                id: resolved_id,
                name,
            })
        }
        #[cfg(not(feature = "wasapi"))]
        {
            let _ = id;
            Err(AudioError::Unsupported)
        }
    }

    /// Returns the first audio device whose name contains `name`
    /// (case-insensitive substring match).
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::DeviceNotFound`] if no matching device is found.
    /// Returns [`AudioError::InitializationFailed`] if COM cannot be
    /// initialised or another lookup failure occurs.
    /// Returns [`AudioError::Unsupported`] when the `wasapi` feature is
    /// not enabled.
    fn from_name(name: &str) -> Result<Self, AudioError> {
        #[cfg(feature = "wasapi")]
        {
            let _com = internal::wasapi::ComGuard::new()?;
            let enumerator = internal::wasapi::create_enumerator()?;
            let devices = internal::wasapi::list_devices(&enumerator)?;

            let needle = name.to_lowercase();
            let (id, matched_name) = devices
                .into_iter()
                .find(|(_, n)| n.to_lowercase().contains(&needle))
                .ok_or(AudioError::DeviceNotFound)?;

            Ok(Self {
                id,
                name: matched_name,
            })
        }
        #[cfg(not(feature = "wasapi"))]
        {
            let _ = name;
            Err(AudioError::Unsupported)
        }
    }

    /// Lists all available audio render devices as `(id, name)` pairs.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::ListFailed`] if the device list cannot be
    /// retrieved.
    /// Returns [`AudioError::InitializationFailed`] if COM cannot be
    /// initialised.
    /// Returns [`AudioError::Unsupported`] when the `wasapi` feature is
    /// not enabled.
    fn list() -> Result<Vec<(String, String)>, AudioError> {
        #[cfg(feature = "wasapi")]
        {
            let _com = internal::wasapi::ComGuard::new()?;
            let enumerator = internal::wasapi::create_enumerator()?;
            internal::wasapi::list_devices(&enumerator)
        }
        #[cfg(not(feature = "wasapi"))]
        Err(AudioError::Unsupported)
    }

    /// Returns the current volume level in the range `0..=100`.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::GetVolumeFailed`] if the volume cannot be read.
    /// Returns [`AudioError::DeviceNotFound`] if this device no longer exists.
    /// Returns [`AudioError::Unsupported`] when the `wasapi` feature is
    /// not enabled.
    fn get_vol(&self) -> Result<u8, AudioError> {
        #[cfg(feature = "wasapi")]
        {
            let _com = internal::wasapi::ComGuard::new()?;
            let enumerator = internal::wasapi::create_enumerator()?;
            let device = internal::wasapi::get_device_by_id(&enumerator, &self.id)?;
            let endpoint = internal::wasapi::endpoint_volume(&device)?;
            internal::wasapi::get_volume(&endpoint)
        }
        #[cfg(not(feature = "wasapi"))]
        Err(AudioError::Unsupported)
    }

    /// Sets the volume level.
    ///
    /// `vol` is clamped to `0..=100` before being applied.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::SetVolumeFailed`] if the volume cannot be set.
    /// Returns [`AudioError::DeviceNotFound`] if this device no longer exists.
    /// Returns [`AudioError::Unsupported`] when the `wasapi` feature is
    /// not enabled.
    fn set_vol(&self, vol: u8) -> Result<(), AudioError> {
        #[cfg(feature = "wasapi")]
        {
            let _com = internal::wasapi::ComGuard::new()?;
            let enumerator = internal::wasapi::create_enumerator()?;
            let device = internal::wasapi::get_device_by_id(&enumerator, &self.id)?;
            let endpoint = internal::wasapi::endpoint_volume(&device)?;
            internal::wasapi::set_volume(&endpoint, vol)
        }
        #[cfg(not(feature = "wasapi"))]
        {
            let _ = vol;
            Err(AudioError::Unsupported)
        }
    }

    /// Returns `true` if the device is currently muted.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::GetMuteFailed`] if the mute state cannot be read.
    /// Returns [`AudioError::DeviceNotFound`] if this device no longer exists.
    /// Returns [`AudioError::Unsupported`] when the `wasapi` feature is
    /// not enabled.
    fn is_mute(&self) -> Result<bool, AudioError> {
        #[cfg(feature = "wasapi")]
        {
            let _com = internal::wasapi::ComGuard::new()?;
            let enumerator = internal::wasapi::create_enumerator()?;
            let device = internal::wasapi::get_device_by_id(&enumerator, &self.id)?;
            let endpoint = internal::wasapi::endpoint_volume(&device)?;
            internal::wasapi::get_mute(&endpoint)
        }
        #[cfg(not(feature = "wasapi"))]
        Err(AudioError::Unsupported)
    }

    /// Mutes or unmutes the device.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::SetMuteFailed`] if the mute state cannot be
    /// changed.
    /// Returns [`AudioError::DeviceNotFound`] if this device no longer exists.
    /// Returns [`AudioError::Unsupported`] when the `wasapi` feature is
    /// not enabled.
    fn set_mute(&self, muted: bool) -> Result<(), AudioError> {
        #[cfg(feature = "wasapi")]
        {
            let _com = internal::wasapi::ComGuard::new()?;
            let enumerator = internal::wasapi::create_enumerator()?;
            let device = internal::wasapi::get_device_by_id(&enumerator, &self.id)?;
            let endpoint = internal::wasapi::endpoint_volume(&device)?;
            internal::wasapi::set_mute(&endpoint, muted)
        }
        #[cfg(not(feature = "wasapi"))]
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

    /// Stub-path tests run on every platform (including the CI Linux runners).
    /// They verify the correct error is returned when the `wasapi` feature is
    /// disabled.

    #[test]
    fn default_returns_unsupported_without_feature() {
        let result = AudioDevice::default();
        assert!(result.is_err());
        #[cfg(not(feature = "wasapi"))]
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[test]
    fn from_id_returns_unsupported_without_feature() {
        let result = AudioDevice::from_id("test-id");
        assert!(result.is_err());
        #[cfg(not(feature = "wasapi"))]
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[test]
    fn from_name_returns_unsupported_without_feature() {
        let result = AudioDevice::from_name("test-name");
        assert!(result.is_err());
        #[cfg(not(feature = "wasapi"))]
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[test]
    fn list_returns_unsupported_without_feature() {
        let result = AudioDevice::list();
        assert!(result.is_err());
        #[cfg(not(feature = "wasapi"))]
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[test]
    fn get_vol_returns_unsupported_without_feature() {
        let device = AudioDevice {
            id: String::from("stub-id"),
            name: String::from("stub-name"),
        };
        let result = device.get_vol();
        assert!(result.is_err());
        #[cfg(not(feature = "wasapi"))]
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[test]
    fn set_vol_returns_unsupported_without_feature() {
        let device = AudioDevice {
            id: String::from("stub-id"),
            name: String::from("stub-name"),
        };
        let result = device.set_vol(50);
        assert!(result.is_err());
        #[cfg(not(feature = "wasapi"))]
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[test]
    fn is_mute_returns_unsupported_without_feature() {
        let device = AudioDevice {
            id: String::from("stub-id"),
            name: String::from("stub-name"),
        };
        let result = device.is_mute();
        assert!(result.is_err());
        #[cfg(not(feature = "wasapi"))]
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[test]
    fn set_mute_returns_unsupported_without_feature() {
        let device = AudioDevice {
            id: String::from("stub-id"),
            name: String::from("stub-name"),
        };
        let result = device.set_mute(true);
        assert!(result.is_err());
        #[cfg(not(feature = "wasapi"))]
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }
}
