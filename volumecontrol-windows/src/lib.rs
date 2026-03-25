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

    // ------------------------------------------------------------------
    // Stub-path tests — only compiled and run when `wasapi` is disabled.
    // ------------------------------------------------------------------

    #[test]
    #[cfg(not(feature = "wasapi"))]
    fn default_returns_unsupported_without_feature() {
        assert!(matches!(
            AudioDevice::default(),
            Err(AudioError::Unsupported)
        ));
    }

    #[test]
    #[cfg(not(feature = "wasapi"))]
    fn from_id_returns_unsupported_without_feature() {
        assert!(matches!(
            AudioDevice::from_id("test-id"),
            Err(AudioError::Unsupported)
        ));
    }

    #[test]
    #[cfg(not(feature = "wasapi"))]
    fn from_name_returns_unsupported_without_feature() {
        assert!(matches!(
            AudioDevice::from_name("test-name"),
            Err(AudioError::Unsupported)
        ));
    }

    #[test]
    #[cfg(not(feature = "wasapi"))]
    fn list_returns_unsupported_without_feature() {
        assert!(matches!(AudioDevice::list(), Err(AudioError::Unsupported)));
    }

    #[test]
    #[cfg(not(feature = "wasapi"))]
    fn get_vol_returns_unsupported_without_feature() {
        let device = AudioDevice {
            id: String::from("stub-id"),
            name: String::from("stub-name"),
        };
        assert!(matches!(device.get_vol(), Err(AudioError::Unsupported)));
    }

    #[test]
    #[cfg(not(feature = "wasapi"))]
    fn set_vol_returns_unsupported_without_feature() {
        let device = AudioDevice {
            id: String::from("stub-id"),
            name: String::from("stub-name"),
        };
        assert!(matches!(device.set_vol(50), Err(AudioError::Unsupported)));
    }

    #[test]
    #[cfg(not(feature = "wasapi"))]
    fn is_mute_returns_unsupported_without_feature() {
        let device = AudioDevice {
            id: String::from("stub-id"),
            name: String::from("stub-name"),
        };
        assert!(matches!(device.is_mute(), Err(AudioError::Unsupported)));
    }

    #[test]
    #[cfg(not(feature = "wasapi"))]
    fn set_mute_returns_unsupported_without_feature() {
        let device = AudioDevice {
            id: String::from("stub-id"),
            name: String::from("stub-name"),
        };
        assert!(matches!(
            device.set_mute(true),
            Err(AudioError::Unsupported)
        ));
    }

    // ------------------------------------------------------------------
    // Real-world WASAPI tests — only compiled and run with `wasapi` feature.
    // These run on Windows CI runners that always have at least one audio
    // endpoint available.
    // ------------------------------------------------------------------

    /// A device ID that is guaranteed to not match any real WASAPI endpoint.
    #[cfg(feature = "wasapi")]
    const BOGUS_ID: &str = "volumecontrol-test-nonexistent-{00000000-0000-0000-0000-000000000000}";

    /// A device name that is guaranteed to not match any real audio device.
    #[cfg(feature = "wasapi")]
    const BOGUS_NAME: &str = "zzz-volumecontrol-test-nonexistent-device-name";

    /// `from_id` with a clearly invalid ID must return `DeviceNotFound` or a
    /// graceful `InitializationFailed` — never a panic or `Unsupported`.
    #[test]
    #[cfg(feature = "wasapi")]
    fn from_id_bogus_returns_not_found() {
        let result = AudioDevice::from_id(BOGUS_ID);
        assert!(
            matches!(
                result,
                Err(AudioError::DeviceNotFound | AudioError::InitializationFailed(_))
            ),
            "expected DeviceNotFound or InitializationFailed, got {result:?}"
        );
    }

    /// `from_name` with a clearly invalid name must return `DeviceNotFound` or
    /// `InitializationFailed` — never a panic or `Unsupported`.
    #[test]
    #[cfg(feature = "wasapi")]
    fn from_name_bogus_returns_not_found() {
        let result = AudioDevice::from_name(BOGUS_NAME);
        assert!(
            matches!(
                result,
                Err(AudioError::DeviceNotFound | AudioError::InitializationFailed(_))
            ),
            "expected DeviceNotFound or InitializationFailed, got {result:?}"
        );
    }

    /// On Windows there is always at least one audio render endpoint; `list()`
    /// must succeed and return a non-empty `Vec`.
    #[test]
    #[cfg(feature = "wasapi")]
    fn list_returns_non_empty_vec() {
        let devices = AudioDevice::list().expect("list() failed on Windows");
        assert!(
            !devices.is_empty(),
            "list() returned an empty Vec on Windows"
        );
    }

    /// On Windows there is always a default audio render endpoint; `default()`
    /// must succeed.
    #[test]
    #[cfg(feature = "wasapi")]
    fn default_device_always_found() {
        AudioDevice::default().expect("default() failed — no default audio device on Windows");
    }

    /// `get_vol` must return a value in `0..=100`; `set_vol` to a different
    /// level must be reflected by the next `get_vol` call.  The original volume
    /// is restored when the test finishes.
    #[test]
    #[cfg(feature = "wasapi")]
    fn default_device_volume_round_trip() {
        let device = AudioDevice::default().expect("default() failed");

        let original_vol = device.get_vol().expect("get_vol() failed");
        assert!(
            original_vol <= 100,
            "get_vol returned {original_vol}, out of range"
        );

        // Pick a target volume that differs from the current one.
        let target_vol: u8 = if original_vol >= 50 { 25 } else { 75 };

        device.set_vol(target_vol).expect("set_vol() failed");

        let new_vol = device.get_vol().expect("get_vol() after set_vol() failed");
        assert_eq!(new_vol, target_vol, "volume did not change to {target_vol}");

        // Restore original volume — best-effort; ignore errors on cleanup.
        let _ = device.set_vol(original_vol);
    }

    /// `set_mute(!original)` must flip the mute state; the change must be
    /// visible via `is_mute`.  The original state is restored afterwards.
    #[test]
    #[cfg(feature = "wasapi")]
    fn default_device_mute_round_trip() {
        let device = AudioDevice::default().expect("default() failed");

        let original = device.is_mute().expect("is_mute() failed");

        // Toggle to the opposite state.
        device.set_mute(!original).expect("set_mute() failed");

        let toggled = device.is_mute().expect("is_mute() after set_mute() failed");
        assert_eq!(
            toggled, !original,
            "mute state did not toggle to {}",
            !original
        );

        // Restore — best-effort; ignore errors on cleanup.
        let _ = device.set_mute(original);
    }
}
