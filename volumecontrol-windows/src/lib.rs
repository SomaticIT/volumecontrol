//! Windows WASAPI volume control backend.
//!
//! This crate exposes an [`AudioDevice`] type that implements
//! [`volumecontrol_core::AudioDevice`].  It exists primarily as an
//! implementation detail of the
//! [`volumecontrol`](https://crates.io/crates/volumecontrol) crate, which
//! selects the correct backend automatically.  If cross-platform support is not
//! a concern you may depend on this crate directly.
//!
//! When the `wasapi` feature is **not** enabled every method returns
//! [`AudioError::Unsupported`], which allows the crate to compile on any
//! platform without the Windows SDK.
//!
//! # Feature flags
//!
//! | Feature  | Description                                    | Requires              |
//! |----------|------------------------------------------------|-----------------------|
//! | `wasapi` | Enable the real WASAPI backend via `windows`    | Windows target only  |
//!
//! # Example
//!
//! ```no_run
//! use volumecontrol_windows::AudioDevice;
//! use volumecontrol_core::AudioDevice as _;
//!
//! fn main() -> Result<(), volumecontrol_core::AudioError> {
//!     let device = AudioDevice::from_default()?;
//!     println!("{device}");  // e.g. "Speakers ({0.0.0.00000000}.{…})"
//!     println!("Current volume: {}%", device.get_vol()?);
//!     Ok(())
//! }
//! ```

#![deny(missing_docs)]

mod internal;

use std::fmt;

use volumecontrol_core::{AudioDevice as AudioDeviceTrait, AudioError, DeviceInfo};

#[cfg(feature = "wasapi")]
use std::sync::Mutex;

#[cfg(feature = "wasapi")]
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;

/// Represents a WASAPI audio output device (Windows).
///
/// # Feature flags
///
/// Real WASAPI integration requires the `wasapi` feature and must be built
/// for a Windows target.  Without the feature every method returns
/// [`AudioError::Unsupported`].
///
/// # Thread safety
///
/// `AudioDevice` is [`Send`] because all COM interface pointers in the
/// `windows` crate are `Send + Sync`: `AddRef` / `Release` are guaranteed to
/// be thread-safe by the COM specification, and `windows-rs` marks every COM
/// interface accordingly.  COM is initialised with `COINIT_MULTITHREADED` (the
/// multi-threaded apartment), so the cached endpoint can be used from any
/// thread in the process without cross-apartment marshalling.
pub struct AudioDevice {
    /// WASAPI endpoint identifier (GUID string).
    id: String,
    /// Friendly device name.
    name: String,
    /// Cached [`IAudioEndpointVolume`] interface.
    ///
    /// Wrapped in a [`Mutex`] to allow transparent re-initialisation on
    /// `AUDCLNT_E_DEVICE_INVALIDATED` errors using only a shared reference
    /// (`&self`).  Only present when the `wasapi` feature is enabled.
    #[cfg(feature = "wasapi")]
    endpoint: Mutex<IAudioEndpointVolume>,
}

impl fmt::Debug for AudioDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The `endpoint` field (a COM interface pointer) is intentionally
        // omitted: it contains no useful human-readable information and
        // exposing raw COM interface addresses in debug output would be
        // confusing.  `finish_non_exhaustive` signals that the struct has
        // additional fields.
        f.debug_struct("AudioDevice")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for AudioDevice {
    /// Formats the device as `"name (id)"`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

#[cfg(feature = "wasapi")]
impl AudioDevice {
    /// Calls `op` with the cached [`IAudioEndpointVolume`], retrying once
    /// after an automatic cache refresh if the endpoint signals
    /// [`EndpointError::DeviceInvalidated`] (`AUDCLNT_E_DEVICE_INVALIDATED`).
    ///
    /// A [`ComGuard`] is created for the duration of the call to ensure COM is
    /// initialised on the calling thread.
    ///
    /// # Errors
    ///
    /// - On `DeviceInvalidated` the cache is refreshed via
    ///   [`try_refresh_endpoint`]; if the refresh itself fails, that error is
    ///   returned.  If the retry still returns `DeviceInvalidated` (device
    ///   disappeared between calls) `AudioError::DeviceNotFound` is returned.
    /// - On any other [`EndpointError::Error`] the wrapped [`AudioError`] is
    ///   propagated unchanged.
    /// - Returns [`AudioError::EndpointLockPoisoned`] if the endpoint mutex is
    ///   poisoned (a thread panicked while holding the lock).
    ///
    /// [`ComGuard`]: internal::wasapi::ComGuard
    /// [`try_refresh_endpoint`]: AudioDevice::try_refresh_endpoint
    fn with_endpoint<T>(
        &self,
        op: impl Fn(&IAudioEndpointVolume) -> Result<T, internal::wasapi::EndpointError>,
    ) -> Result<T, AudioError> {
        let _com = internal::wasapi::ComGuard::new()?;
        let guard = self
            .endpoint
            .lock()
            .map_err(|_| AudioError::EndpointLockPoisoned)?;
        match op(&guard) {
            Ok(v) => Ok(v),
            Err(internal::wasapi::EndpointError::Error(e)) => Err(e),
            Err(internal::wasapi::EndpointError::DeviceInvalidated) => {
                // Release the lock before refreshing so `try_refresh_endpoint`
                // can also acquire it.
                drop(guard);
                // AUDCLNT_E_DEVICE_INVALIDATED — refresh cache and retry once.
                self.try_refresh_endpoint()?;
                let guard = self
                    .endpoint
                    .lock()
                    .map_err(|_| AudioError::EndpointLockPoisoned)?;
                match op(&guard) {
                    Ok(v) => Ok(v),
                    Err(internal::wasapi::EndpointError::Error(e)) => Err(e),
                    // Still invalidated after a fresh endpoint: device is gone.
                    Err(internal::wasapi::EndpointError::DeviceInvalidated) => {
                        Err(AudioError::DeviceNotFound)
                    }
                }
            }
        }
    }

    /// Re-resolves the device by its cached ID and replaces the stored
    /// [`IAudioEndpointVolume`] with a freshly activated one.
    ///
    /// Called by [`with_endpoint`] when an endpoint operation returns
    /// [`EndpointError::DeviceInvalidated`]
    /// (`AUDCLNT_E_DEVICE_INVALIDATED`).
    /// The caller is responsible for ensuring COM is already initialised on the
    /// current thread (i.e. a [`ComGuard`] is alive in the calling scope).
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::DeviceNotFound`] if the device no longer exists,
    /// [`AudioError::InitializationFailed`] on other COM failures, or
    /// [`AudioError::EndpointLockPoisoned`] if the endpoint mutex is poisoned.
    ///
    /// [`with_endpoint`]: AudioDevice::with_endpoint
    /// [`ComGuard`]: internal::wasapi::ComGuard
    fn try_refresh_endpoint(&self) -> Result<(), AudioError> {
        let enumerator = internal::wasapi::create_enumerator()?;
        let device = internal::wasapi::get_device_by_id(&enumerator, &self.id)?;
        let new_endpoint = internal::wasapi::endpoint_volume(&device)?;
        *self
            .endpoint
            .lock()
            .map_err(|_| AudioError::EndpointLockPoisoned)? = new_endpoint;
        Ok(())
    }
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
    fn from_default() -> Result<Self, AudioError> {
        #[cfg(feature = "wasapi")]
        {
            let _com = internal::wasapi::ComGuard::new()?;
            let enumerator = internal::wasapi::create_enumerator()?;
            let device = internal::wasapi::get_default_device(&enumerator)?;
            let id = internal::wasapi::device_id(&device)?;
            let name = internal::wasapi::device_name(&device)?;
            let endpoint = internal::wasapi::endpoint_volume(&device)?;
            Ok(Self {
                id,
                name,
                endpoint: Mutex::new(endpoint),
            })
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
            let endpoint = internal::wasapi::endpoint_volume(&device)?;
            Ok(Self {
                id: resolved_id,
                name,
                endpoint: Mutex::new(endpoint),
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
            let info = devices
                .into_iter()
                .find(|d| d.name.to_lowercase().contains(&needle))
                .ok_or(AudioError::DeviceNotFound)?;

            // Re-resolve the IMMDevice from its ID to activate the endpoint.
            let device = internal::wasapi::get_device_by_id(&enumerator, &info.id)?;
            let endpoint = internal::wasapi::endpoint_volume(&device)?;

            Ok(Self {
                id: info.id,
                name: info.name,
                endpoint: Mutex::new(endpoint),
            })
        }
        #[cfg(not(feature = "wasapi"))]
        {
            let _ = name;
            Err(AudioError::Unsupported)
        }
    }

    /// Lists all available audio render devices.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::ListFailed`] if the device list cannot be
    /// retrieved.
    /// Returns [`AudioError::InitializationFailed`] if COM cannot be
    /// initialised.
    /// Returns [`AudioError::Unsupported`] when the `wasapi` feature is
    /// not enabled.
    fn list() -> Result<Vec<DeviceInfo>, AudioError> {
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
            self.with_endpoint(internal::wasapi::get_volume)
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
            self.with_endpoint(|ep| internal::wasapi::set_volume(ep, vol))
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
            self.with_endpoint(internal::wasapi::get_mute)
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
            self.with_endpoint(|ep| internal::wasapi::set_mute(ep, muted))
        }
        #[cfg(not(feature = "wasapi"))]
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

    /// `Display` output must follow the `"name (id)"` format.
    ///
    /// The test is gated on `not(wasapi)` because constructing an
    /// [`AudioDevice`] directly without a valid COM endpoint is only safe when
    /// the `wasapi` feature is disabled (no `endpoint` field exists).
    #[test]
    #[cfg(not(feature = "wasapi"))]
    fn display_format_is_name_paren_id() {
        let device = AudioDevice {
            id: "{0.0.0.00000000}.{E9B0A576-1234-5678-ABCD-000000000000}".to_string(),
            name: "Speakers".to_string(),
        };
        assert_eq!(
            device.to_string(),
            "Speakers ({0.0.0.00000000}.{E9B0A576-1234-5678-ABCD-000000000000})"
        );
    }

    // ------------------------------------------------------------------
    // Stub-path tests — only compiled and run when `wasapi` is disabled.
    // ------------------------------------------------------------------

    #[test]
    #[cfg(not(feature = "wasapi"))]
    fn default_returns_unsupported_without_feature() {
        assert!(matches!(
            AudioDevice::from_default(),
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

    /// `from_name` must match regardless of the case of the query string.
    #[test]
    #[cfg(feature = "wasapi")]
    fn from_name_case_insensitive_match_returns_ok() {
        let default_device = AudioDevice::from_default().expect("from_default() failed");
        let upper = default_device.name().to_uppercase();
        let found = AudioDevice::from_name(&upper);
        assert!(
            found.is_ok(),
            "from_name with uppercase query '{upper}' should succeed (case-insensitive)"
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
        AudioDevice::from_default()
            .expect("from_default() failed — no default audio device on Windows");
    }

    /// `get_vol` must return a value in `0..=100`; `set_vol` to a different
    /// level must be reflected by the next `get_vol` call.  The original volume
    /// is restored when the test finishes.
    #[test]
    #[cfg(feature = "wasapi")]
    fn default_device_volume_round_trip() {
        let device = AudioDevice::from_default().expect("from_default() failed");

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
        let device = AudioDevice::from_default().expect("from_default() failed");

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
