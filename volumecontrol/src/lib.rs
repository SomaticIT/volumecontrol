//! Cross-platform crate to control system audio volume.
//!
//! This crate provides a unified [`AudioDevice`] type that works on Linux,
//! Windows, and macOS. The correct backend is selected automatically at
//! compile time; no feature flags or sub-crate imports are required.
//!
//! | Platform | Backend              | System library required |
//! |----------|----------------------|-------------------------|
//! | Linux    | PulseAudio           | `libpulse-dev`          |
//! | Windows  | WASAPI               | built-in                |
//! | macOS    | CoreAudio            | built-in                |
//!
//! # Example
//!
//! ```no_run
//! use volumecontrol::AudioDevice;
//!
//! fn main() -> Result<(), volumecontrol::AudioError> {
//!     let device = AudioDevice::from_default()?;
//!     println!("{device}");  // e.g. "Speakers ({0.0.0.00000000}.{…})"
//!     println!("Current volume: {}%", device.get_vol()?);
//!     Ok(())
//! }
//! ```

#![deny(missing_docs)]

use std::fmt;

pub use volumecontrol_core::AudioError;
pub use volumecontrol_core::DeviceInfo;

use volumecontrol_core::AudioDevice as _;

#[cfg(target_os = "linux")]
use volumecontrol_linux::AudioDevice as Inner;

#[cfg(target_os = "windows")]
use volumecontrol_windows::AudioDevice as Inner;

#[cfg(target_os = "macos")]
use volumecontrol_macos::AudioDevice as Inner;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
compile_error!(
    "volumecontrol does not support the current target OS. \
     Supported targets: linux, windows, macos."
);

/// A cross-platform audio output device.
///
/// Wraps the platform-appropriate backend and exposes a uniform API for
/// querying and changing the system volume and mute state.  No trait imports
/// are required to use the methods below.
#[derive(Debug)]
pub struct AudioDevice(Inner);

impl fmt::Display for AudioDevice {
    /// Formats the device by delegating to the inner backend.
    ///
    /// The conventional format is `"name (id)"`,
    /// e.g. `"Speakers ({0.0.0.00000000}.{…})"`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AudioDevice {
    /// Returns the system default audio output device.
    ///
    /// # Errors
    ///
    /// Returns an error if the default device cannot be resolved.
    pub fn from_default() -> Result<Self, AudioError> {
        Inner::from_default().map(Self)
    }

    /// Returns the audio device identified by `id`.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::DeviceNotFound`] if no device with the given
    /// identifier exists, or another error if the lookup fails.
    pub fn from_id(id: &str) -> Result<Self, AudioError> {
        Inner::from_id(id).map(Self)
    }

    /// Returns the first audio device whose name contains `name`.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::DeviceNotFound`] if no matching device is found,
    /// or another error if the lookup fails.
    pub fn from_name(name: &str) -> Result<Self, AudioError> {
        Inner::from_name(name).map(Self)
    }

    /// Lists all available audio devices.
    ///
    /// # Errors
    ///
    /// Returns an error if the device list cannot be retrieved.
    pub fn list() -> Result<Vec<DeviceInfo>, AudioError> {
        Inner::list()
    }

    /// Returns the current volume level in the range `0..=100`.
    ///
    /// # Errors
    ///
    /// Returns an error if the volume cannot be read.
    pub fn get_vol(&self) -> Result<u8, AudioError> {
        self.0.get_vol()
    }

    /// Sets the volume level.
    ///
    /// `vol` is clamped to `0..=100` before being applied.
    ///
    /// # Errors
    ///
    /// Returns an error if the volume cannot be set.
    pub fn set_vol(&self, vol: u8) -> Result<(), AudioError> {
        self.0.set_vol(vol)
    }

    /// Returns `true` if the device is currently muted.
    ///
    /// # Errors
    ///
    /// Returns an error if the mute state cannot be read.
    pub fn is_mute(&self) -> Result<bool, AudioError> {
        self.0.is_mute()
    }

    /// Mutes or unmutes the device.
    ///
    /// # Errors
    ///
    /// Returns an error if the mute state cannot be changed.
    pub fn set_mute(&self, muted: bool) -> Result<(), AudioError> {
        self.0.set_mute(muted)
    }

    /// Returns the unique identifier for this device.
    ///
    /// The value is the same opaque string that [`Self::list`] yields as
    /// [`DeviceInfo::id`] and that [`Self::from_id`]
    /// accepts as its argument.  It is guaranteed to be non-empty.
    ///
    /// # Platform-specific formats
    ///
    /// | Platform | Format                                                  |
    /// |----------|---------------------------------------------------------|
    /// | Linux    | PulseAudio sink name (e.g. `alsa_output.pci-0000_…`)    |
    /// | Windows  | WASAPI endpoint ID (e.g. `{0.0.0.00000000}.{…}`)       |
    /// | macOS    | CoreAudio device UID (numeric string, e.g. `"73"`)      |
    pub fn id(&self) -> &str {
        self.0.id()
    }

    /// Returns the human-readable display name of this device.
    ///
    /// The value is the same string that [`Self::list`] yields as
    /// [`DeviceInfo::name`] and that [`Self::from_name`] uses for
    /// substring matching.  It is guaranteed to be non-empty.
    ///
    /// # Platform-specific formats
    ///
    /// | Platform | Format                                                  |
    /// |----------|---------------------------------------------------------|
    /// | Linux    | PulseAudio sink description (e.g. `"Built-in Audio"`)   |
    /// | Windows  | WASAPI endpoint friendly name (e.g. `"Speakers"`)       |
    /// | macOS    | CoreAudio device name (e.g. `"MacBook Pro Speakers"`)   |
    pub fn name(&self) -> &str {
        self.0.name()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // A bogus device id guaranteed not to match any real device.
    // The format matches what each backend considers an invalid lookup:
    // - Windows: a GUID-style path that no WASAPI endpoint will carry
    // - macOS: a non-numeric string (CoreAudio ids are integers)
    // - Linux / other: a PulseAudio sink name that cannot exist
    #[cfg(target_os = "windows")]
    const BOGUS_ID: &str = "volumecontrol-test-nonexistent-{00000000-0000-0000-0000-000000000000}";
    #[cfg(target_os = "macos")]
    const BOGUS_ID: &str = "not-a-number";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    const BOGUS_ID: &str = "__nonexistent_sink_xyz__";

    // A bogus device name guaranteed not to match any real audio device.
    const BOGUS_NAME: &str = "zzz-volumecontrol-test-nonexistent-device-name";

    /// `Display` output for the default device must follow `"name (id)"`.
    #[test]
    fn display_contains_name_and_id() {
        let device = AudioDevice::from_default().expect("from_default()");
        let s = device.to_string();
        assert!(
            s.contains(device.name()),
            "Display output should contain the device name; got: {s}"
        );
        assert!(
            s.contains(device.id()),
            "Display output should contain the device id; got: {s}"
        );
        // Verify the exact "name (id)" format.
        let expected = format!("{} ({})", device.name(), device.id());
        assert_eq!(s, expected);
    }

    /// The default device must have a non-empty id and name.
    #[test]
    fn default_device_id_and_name_nonempty() {
        let device = AudioDevice::from_default().expect("from_default()");
        assert!(!device.id().is_empty(), "device id must not be empty");
        assert!(!device.name().is_empty(), "device name must not be empty");
    }

    /// The default device must be resolvable when an audio device is present.
    #[test]
    fn default_returns_ok() {
        let result = AudioDevice::from_default();
        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }

    /// `list()` must return at least one device with non-empty id and name.
    #[test]
    fn list_returns_nonempty() {
        let devices = AudioDevice::list().expect("list()");
        assert!(
            !devices.is_empty(),
            "expected at least one audio device from list()"
        );
        for info in &devices {
            assert!(!info.id.is_empty(), "device id must not be empty");
            assert!(!info.name.is_empty(), "device name must not be empty");
        }
    }

    /// Looking up a device by id obtained from `list()` must succeed.
    #[test]
    fn from_id_valid_id_returns_ok() {
        let devices = AudioDevice::list().expect("list()");
        let first = devices.first().expect("at least one device in list");
        let found = AudioDevice::from_id(&first.id);
        assert!(
            found.is_ok(),
            "from_id with a valid id should succeed, got {found:?}"
        );
    }

    /// A bogus device id must return an error.
    #[test]
    fn from_id_nonexistent_returns_err() {
        let result = AudioDevice::from_id(BOGUS_ID);
        assert!(result.is_err(), "expected an error, got {result:?}");
    }

    /// A partial description substring of a listed device must match.
    #[test]
    fn from_name_partial_match_returns_ok() {
        let devices = AudioDevice::list().expect("list()");
        let first = devices.first().expect("at least one device in list");
        let partial: String = first.name.chars().take(3).collect();
        let found = AudioDevice::from_name(&partial);
        assert!(
            found.is_ok(),
            "from_name with partial match '{partial}' should succeed"
        );
    }

    /// A name that matches no device must return an error.
    #[test]
    fn from_name_no_match_returns_err() {
        let result = AudioDevice::from_name(BOGUS_NAME);
        assert!(result.is_err(), "expected an error, got {result:?}");
    }

    /// The reported volume must always be within the valid `0..=100` range.
    #[test]
    fn get_vol_returns_valid_range() {
        let device = AudioDevice::from_default().expect("from_default()");
        let vol = device.get_vol().expect("get_vol()");
        assert!(vol <= 100, "volume must be in 0..=100, got {vol}");
    }

    /// Setting the volume to a different value must be reflected when read back.
    ///
    /// The original volume is restored so that other tests are not affected.
    /// Run with `--test-threads=1` to avoid races.
    #[test]
    fn set_vol_changes_volume() {
        let device = AudioDevice::from_default().expect("from_default()");
        let original = device.get_vol().expect("get_vol()");
        let target: u8 = if original >= 50 { 30 } else { 70 };
        device.set_vol(target).expect("set_vol()");
        let after = device.get_vol().expect("get_vol() after set");
        // Allow ±1 rounding error due to floating-point ↔ integer conversion.
        assert!(
            after.abs_diff(target) <= 1,
            "expected volume near {target}, got {after}"
        );
        device.set_vol(original).expect("restore original volume");
    }

    /// Toggling the mute state must be reflected when read back.
    ///
    /// The original mute state is restored so that other tests are not affected.
    /// Run with `--test-threads=1` to avoid races.
    #[test]
    fn set_mute_changes_mute_state() {
        let device = AudioDevice::from_default().expect("from_default()");
        let original = device.is_mute().expect("is_mute()");
        let target = !original;
        device.set_mute(target).expect("set_mute()");
        let after = device.is_mute().expect("is_mute() after set");
        assert_eq!(after, target, "mute state should be {target}, got {after}");
        device
            .set_mute(original)
            .expect("restore original mute state");
    }
}
