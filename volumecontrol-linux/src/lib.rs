//! Linux PulseAudio volume control backend.
//!
//! This crate exposes an [`AudioDevice`] type that implements
//! [`volumecontrol_core::AudioDevice`].  It exists primarily as an
//! implementation detail of the
//! [`volumecontrol`](https://crates.io/crates/volumecontrol) crate, which
//! selects the correct backend automatically.  If cross-platform support is not
//! a concern you may depend on this crate directly.
//!
//! When the `pulseaudio` feature is **not** enabled every method returns
//! [`AudioError::Unsupported`], which allows the crate to compile on any
//! platform without the PulseAudio development headers.
//!
//! # Feature flags
//!
//! | Feature      | Description                                             | Requires              |
//! |--------------|---------------------------------------------------------|-----------------------|
//! | `pulseaudio` | Enable the real PulseAudio backend via `libpulse-binding` | `libpulse-dev` system package |
//!
//! # Example
//!
//! ```no_run
//! use volumecontrol_linux::AudioDevice;
//! use volumecontrol_core::AudioDevice as _;
//!
//! fn main() -> Result<(), volumecontrol_core::AudioError> {
//!     let device = AudioDevice::from_default()?;
//!     println!("{device}");  // e.g. "Built-in Audio (alsa_output.pci-…)"
//!     println!("Current volume: {}%", device.get_vol()?);
//!     Ok(())
//! }
//! ```

#![deny(missing_docs)]

use std::fmt;

use volumecontrol_core::{AudioDevice as AudioDeviceTrait, AudioError, DeviceInfo};

#[cfg(feature = "pulseaudio")]
use std::{cell::RefCell, rc::Rc};

#[cfg(feature = "pulseaudio")]
mod pulse;

/// Represents a PulseAudio audio output device.
///
/// # Feature flags
///
/// Real PulseAudio integration requires the `pulseaudio` feature and the
/// `libpulse-dev` system package.  Without the feature every method returns
/// [`AudioError::Unsupported`].
///
/// # Thread safety
///
/// When the `pulseaudio` feature is enabled, `AudioDevice` is **not** `Send`
/// because it holds a cached PulseAudio connection (`Mainloop` and
/// `Context` from `libpulse-binding` are `!Send`).  Use on a single thread
/// only.  A threaded-mainloop wrapper that restores `Send + Sync` may be
/// added in a future release.
pub struct AudioDevice {
    /// PulseAudio sink name used as the unique device identifier.
    id: String,
    /// Human-readable sink description.
    name: String,
    /// Cached PulseAudio connection, lazily initialised on first use and
    /// reconnected automatically after a server disconnect.
    ///
    /// `None` only when the struct is built directly in tests (bypassing the
    /// constructors); in that case the first method call will try to connect.
    #[cfg(feature = "pulseaudio")]
    conn: Rc<RefCell<Option<pulse::PulseConnection>>>,
}

impl fmt::Debug for AudioDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

#[cfg(feature = "pulseaudio")]
impl AudioDevice {
    /// Returns a mutable reference to the cached [`pulse::PulseConnection`],
    /// creating a fresh connection if the slot is empty.
    ///
    /// Each [`pulse::PulseConnection`] method already calls `ensure_ready()`
    /// internally, so callers do not need to handle reconnection themselves.
    fn get_or_connect(
        opt: &mut Option<pulse::PulseConnection>,
    ) -> Result<&mut pulse::PulseConnection, AudioError> {
        if opt.is_none() {
            *opt = Some(pulse::PulseConnection::new()?);
        }
        opt.as_mut()
            .ok_or_else(|| AudioError::InitializationFailed("connection slot was empty".into()))
    }
}

impl AudioDeviceTrait for AudioDevice {
    fn from_default() -> Result<Self, AudioError> {
        #[cfg(feature = "pulseaudio")]
        {
            let mut conn = pulse::PulseConnection::new()?;
            let sink_name = conn.default_sink_name()?;
            let snap = conn.sink_by_name(&sink_name)?;
            Ok(AudioDevice {
                id: snap.name,
                name: snap.description,
                conn: Rc::new(RefCell::new(Some(conn))),
            })
        }
        #[cfg(not(feature = "pulseaudio"))]
        Err(AudioError::Unsupported)
    }

    fn from_id(id: &str) -> Result<Self, AudioError> {
        #[cfg(feature = "pulseaudio")]
        {
            let mut conn = pulse::PulseConnection::new()?;
            let snap = conn.sink_by_name(id)?;
            Ok(AudioDevice {
                id: snap.name,
                name: snap.description,
                conn: Rc::new(RefCell::new(Some(conn))),
            })
        }
        #[cfg(not(feature = "pulseaudio"))]
        {
            let _ = id;
            Err(AudioError::Unsupported)
        }
    }

    fn from_name(name: &str) -> Result<Self, AudioError> {
        #[cfg(feature = "pulseaudio")]
        {
            let mut conn = pulse::PulseConnection::new()?;
            let snap = conn.sink_matching_description(name)?;
            Ok(AudioDevice {
                id: snap.name,
                name: snap.description,
                conn: Rc::new(RefCell::new(Some(conn))),
            })
        }
        #[cfg(not(feature = "pulseaudio"))]
        {
            let _ = name;
            Err(AudioError::Unsupported)
        }
    }

    fn list() -> Result<Vec<DeviceInfo>, AudioError> {
        #[cfg(feature = "pulseaudio")]
        {
            pulse::PulseConnection::new()?.list_sinks()
        }
        #[cfg(not(feature = "pulseaudio"))]
        Err(AudioError::Unsupported)
    }

    fn get_vol(&self) -> Result<u8, AudioError> {
        #[cfg(feature = "pulseaudio")]
        {
            let mut guard = self.conn.borrow_mut();
            let conn = Self::get_or_connect(&mut guard)?;
            Ok(conn.sink_by_name(&self.id)?.volume)
        }
        #[cfg(not(feature = "pulseaudio"))]
        Err(AudioError::Unsupported)
    }

    fn set_vol(&self, vol: u8) -> Result<(), AudioError> {
        #[cfg(feature = "pulseaudio")]
        {
            let mut guard = self.conn.borrow_mut();
            let conn = Self::get_or_connect(&mut guard)?;
            conn.set_sink_volume(&self.id, vol)
        }
        #[cfg(not(feature = "pulseaudio"))]
        {
            let _ = vol;
            Err(AudioError::Unsupported)
        }
    }

    fn is_mute(&self) -> Result<bool, AudioError> {
        #[cfg(feature = "pulseaudio")]
        {
            let mut guard = self.conn.borrow_mut();
            let conn = Self::get_or_connect(&mut guard)?;
            Ok(conn.sink_by_name(&self.id)?.mute)
        }
        #[cfg(not(feature = "pulseaudio"))]
        Err(AudioError::Unsupported)
    }

    fn set_mute(&self, muted: bool) -> Result<(), AudioError> {
        #[cfg(feature = "pulseaudio")]
        {
            let mut guard = self.conn.borrow_mut();
            let conn = Self::get_or_connect(&mut guard)?;
            conn.set_sink_mute(&self.id, muted)
        }
        #[cfg(not(feature = "pulseaudio"))]
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
    #[test]
    fn display_format_is_name_paren_id() {
        let device = AudioDevice {
            id: "alsa_output.pci-0000_00_1b.0.analog-stereo".to_string(),
            name: "Built-in Audio Analog Stereo".to_string(),
            #[cfg(feature = "pulseaudio")]
            conn: std::rc::Rc::new(std::cell::RefCell::new(None)),
        };
        assert_eq!(
            device.to_string(),
            "Built-in Audio Analog Stereo (alsa_output.pci-0000_00_1b.0.analog-stereo)"
        );
    }

    #[cfg(not(feature = "pulseaudio"))]
    #[test]
    fn default_returns_unsupported_without_feature() {
        let result = AudioDevice::from_default();
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[cfg(not(feature = "pulseaudio"))]
    #[test]
    fn from_id_returns_unsupported_without_feature() {
        let result = AudioDevice::from_id("test-id");
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[cfg(not(feature = "pulseaudio"))]
    #[test]
    fn from_name_returns_unsupported_without_feature() {
        let result = AudioDevice::from_name("test-name");
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    #[cfg(not(feature = "pulseaudio"))]
    #[test]
    fn list_returns_unsupported_without_feature() {
        let result = AudioDevice::list();
        assert!(matches!(result.unwrap_err(), AudioError::Unsupported));
    }

    /// When the `pulseaudio` feature is disabled, every `&self` method on an
    /// `AudioDevice` must return `Err(AudioError::Unsupported)`.
    #[cfg(not(feature = "pulseaudio"))]
    #[test]
    fn self_methods_return_unsupported_without_feature() {
        // Construct a dummy device directly; the public constructors also
        // return `Unsupported` without the feature.
        let device = AudioDevice {
            id: String::new(),
            name: String::new(),
        };
        assert!(matches!(
            device.get_vol().unwrap_err(),
            AudioError::Unsupported
        ));
        assert!(matches!(
            device.set_vol(50).unwrap_err(),
            AudioError::Unsupported
        ));
        assert!(matches!(
            device.is_mute().unwrap_err(),
            AudioError::Unsupported
        ));
        assert!(matches!(
            device.set_mute(false).unwrap_err(),
            AudioError::Unsupported
        ));
    }

    // ── Tests for the `pulseaudio` feature ───────────────────────────────────
    //
    // These tests do not require a running PulseAudio server.  When no server
    // is available every method that opens a connection returns
    // `Err(AudioError::InitializationFailed(_))`.  When a server is running
    // but the requested resource does not exist the constructors return
    // `Err(AudioError::DeviceNotFound)`.

    /// Looks up a sink ID that is guaranteed not to exist.
    /// Expects `DeviceNotFound` (server running, no such sink) or
    /// `InitializationFailed` (no server running).
    #[cfg(feature = "pulseaudio")]
    #[test]
    fn from_id_fails_for_nonexistent_sink() {
        let result = AudioDevice::from_id("__nonexistent_sink_xyz__");
        assert!(result.is_err(), "expected an error, got Ok");
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                AudioError::DeviceNotFound | AudioError::InitializationFailed(_)
            ),
            "unexpected error variant: {err:?}"
        );
    }

    /// Searches by a description that is guaranteed not to match any sink.
    #[cfg(feature = "pulseaudio")]
    #[test]
    fn from_name_fails_for_nonexistent_description() {
        let result = AudioDevice::from_name("__nonexistent_description_xyz__");
        assert!(result.is_err(), "expected an error, got Ok");
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                AudioError::DeviceNotFound | AudioError::InitializationFailed(_)
            ),
            "unexpected error variant: {err:?}"
        );
    }

    /// `list()` must either succeed (returns `Ok`) or fail with
    /// `InitializationFailed` — it must never panic or return an unexpected
    /// error variant.
    #[cfg(feature = "pulseaudio")]
    #[test]
    fn list_returns_ok_or_init_failed() {
        let result = AudioDevice::list();
        match &result {
            Ok(_) => {}
            Err(AudioError::InitializationFailed(_)) => {}
            Err(e) => panic!("unexpected error from list(): {e:?}"),
        }
    }

    /// `from_default()` must either succeed, return `DeviceNotFound` (no default
    /// sink configured), or return `InitializationFailed` (no server).
    #[cfg(feature = "pulseaudio")]
    #[test]
    fn default_returns_ok_or_known_error() {
        let result = AudioDevice::from_default();
        match &result {
            Ok(_) => {}
            Err(AudioError::InitializationFailed(_)) | Err(AudioError::DeviceNotFound) => {}
            Err(e) => panic!("unexpected error from from_default(): {e:?}"),
        }
    }

    /// `get_vol`, `is_mute`, and `set_vol` on a device whose sink ID does not
    /// exist return `DeviceNotFound` (server running) or `InitializationFailed`
    /// (no server).
    ///
    /// The device is constructed with `conn: None` so that the first method
    /// call will attempt to connect (and fail gracefully if no server is
    /// present).
    #[cfg(feature = "pulseaudio")]
    #[test]
    fn self_methods_fail_for_nonexistent_sink() {
        let device = AudioDevice {
            id: "__nonexistent_sink_xyz__".to_string(),
            name: String::new(),
            conn: Rc::new(RefCell::new(None)),
        };

        let result = device.get_vol();
        assert!(result.is_err(), "get_vol: expected error, got Ok");
        assert!(
            matches!(
                result.unwrap_err(),
                AudioError::DeviceNotFound | AudioError::InitializationFailed(_)
            ),
            "get_vol: unexpected error variant"
        );

        let result = device.is_mute();
        assert!(result.is_err(), "is_mute: expected error, got Ok");
        assert!(
            matches!(
                result.unwrap_err(),
                AudioError::DeviceNotFound | AudioError::InitializationFailed(_)
            ),
            "is_mute: unexpected error variant"
        );

        // set_vol fetches the current ChannelVolumes first (via sink_by_name),
        // so a missing sink surfaces as DeviceNotFound before any write.
        let result = device.set_vol(50);
        assert!(result.is_err(), "set_vol: expected error, got Ok");
        assert!(
            matches!(
                result.unwrap_err(),
                AudioError::DeviceNotFound | AudioError::InitializationFailed(_)
            ),
            "set_vol: unexpected error variant"
        );
    }

    // ── real-world tests (pulseaudio feature, Linux only) ─────────────────────
    //
    // These tests exercise the actual PulseAudio stack and therefore require a
    // running PulseAudio server with at least one available sink.  In CI a
    // virtual null sink is provisioned before this test suite runs.

    /// The default device must be resolvable when PulseAudio is running.
    #[cfg(all(feature = "pulseaudio", target_os = "linux"))]
    #[test]
    fn default_returns_ok() {
        let device = AudioDevice::from_default();
        assert!(device.is_ok(), "expected Ok, got {device:?}");
    }

    /// `list()` must return at least one device, each with a non-empty id and name.
    #[cfg(all(feature = "pulseaudio", target_os = "linux"))]
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

    /// Looking up the default device by its sink name must succeed and return
    /// the same id.
    #[cfg(all(feature = "pulseaudio", target_os = "linux"))]
    #[test]
    fn from_id_valid_id_returns_ok() {
        let default_device = AudioDevice::from_default().expect("from_default()");
        let found_device = match AudioDevice::from_id(default_device.id()) {
            Ok(d) => d,
            Err(e) => panic!("from_id with valid id should succeed, got {e:?}"),
        };
        assert_eq!(found_device.id(), default_device.id());
    }

    /// A sink name that does not exist must return `DeviceNotFound`.
    #[cfg(all(feature = "pulseaudio", target_os = "linux"))]
    #[test]
    fn from_id_nonexistent_returns_not_found() {
        let result = AudioDevice::from_id("__nonexistent_sink_xyz__");
        match result {
            Err(AudioError::DeviceNotFound) => {}
            other => panic!("expected DeviceNotFound, got {other:?}"),
        }
    }

    /// A partial description substring of the default device must match.
    #[cfg(all(feature = "pulseaudio", target_os = "linux"))]
    #[test]
    fn from_name_partial_match_returns_ok() {
        let default_device = AudioDevice::from_default().expect("from_default()");
        let partial: String = default_device.name().chars().take(3).collect();
        let found = AudioDevice::from_name(&partial);
        assert!(
            found.is_ok(),
            "from_name with partial match '{partial}' should succeed"
        );
    }

    /// `from_name` must match regardless of the case of the query string.
    #[cfg(all(feature = "pulseaudio", target_os = "linux"))]
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

    /// A description that matches no sink must return `DeviceNotFound`.
    #[cfg(all(feature = "pulseaudio", target_os = "linux"))]
    #[test]
    fn from_name_no_match_returns_not_found() {
        let result = AudioDevice::from_name("\x00\x01\x02");
        match result {
            Err(AudioError::DeviceNotFound) => {}
            other => panic!("expected DeviceNotFound, got {other:?}"),
        }
    }

    /// The reported volume must always be within the valid `0..=100` range.
    #[cfg(all(feature = "pulseaudio", target_os = "linux"))]
    #[test]
    fn get_vol_returns_valid_range() {
        let device = AudioDevice::from_default().expect("from_default()");
        let vol = device.get_vol().expect("get_vol()");
        assert!(vol <= 100, "volume must be in 0..=100, got {vol}");
    }

    /// Setting the volume to a different value must be reflected when read back.
    ///
    /// The original volume is restored at the end of the test so that other
    /// tests are not affected (run with `--test-threads=1` to avoid races).
    #[cfg(all(feature = "pulseaudio", target_os = "linux"))]
    #[test]
    fn set_vol_changes_volume() {
        let device = AudioDevice::from_default().expect("from_default()");
        let original = device.get_vol().expect("get_vol()");
        // Choose a target value that is clearly different from the original.
        let target: u8 = if original >= 50 { 30 } else { 70 };
        device.set_vol(target).expect("set_vol()");
        let after = device.get_vol().expect("get_vol() after set");
        // Allow ±1 rounding error due to f32 ↔ u8 conversion.
        assert!(
            after.abs_diff(target) <= 1,
            "expected volume near {target}, got {after}"
        );
        // Restore the original volume.
        device.set_vol(original).expect("restore original volume");
    }

    /// Toggling the mute state must be reflected when read back.
    ///
    /// The original mute state is restored at the end of the test so that
    /// other tests are not affected (run with `--test-threads=1` to avoid races).
    #[cfg(all(feature = "pulseaudio", target_os = "linux"))]
    #[test]
    fn set_mute_changes_mute_state() {
        let device = AudioDevice::from_default().expect("from_default()");
        let original = device.is_mute().expect("is_mute()");
        // Toggle to the opposite state.
        let target = !original;
        device.set_mute(target).expect("set_mute()");
        let after = device.is_mute().expect("is_mute() after set");
        assert_eq!(after, target, "mute state should be {target}, got {after}");
        // Restore the original mute state.
        device
            .set_mute(original)
            .expect("restore original mute state");
    }
}
