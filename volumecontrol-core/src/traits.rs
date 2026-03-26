use crate::AudioError;

/// Common interface for audio device volume control.
///
/// Implementors represent a single audio output device and expose uniform
/// methods for querying and changing its volume and mute state.
pub trait AudioDevice: Sized {
    /// Returns the system default audio output device.
    ///
    /// # Errors
    ///
    /// Returns an error if the default device cannot be resolved.
    fn from_default() -> Result<Self, AudioError>;

    /// Returns the audio device identified by `id`.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::DeviceNotFound`] if no device with the given
    /// identifier exists, or another error if the lookup fails.
    fn from_id(id: &str) -> Result<Self, AudioError>;

    /// Returns the first audio device whose name contains `name`.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::DeviceNotFound`] if no matching device is found,
    /// or another error if the lookup fails.
    fn from_name(name: &str) -> Result<Self, AudioError>;

    /// Lists all available audio devices as `(id, name)` pairs.
    ///
    /// # Errors
    ///
    /// Returns an error if the device list cannot be retrieved.
    fn list() -> Result<Vec<(String, String)>, AudioError>;

    /// Returns the current volume level in the range `0..=100`.
    ///
    /// # Errors
    ///
    /// Returns an error if the volume cannot be read.
    fn get_vol(&self) -> Result<u8, AudioError>;

    /// Sets the volume level.
    ///
    /// `vol` is clamped to `0..=100` before being applied.
    ///
    /// # Errors
    ///
    /// Returns an error if the volume cannot be set.
    fn set_vol(&self, vol: u8) -> Result<(), AudioError>;

    /// Returns `true` if the device is currently muted.
    ///
    /// # Errors
    ///
    /// Returns an error if the mute state cannot be read.
    fn is_mute(&self) -> Result<bool, AudioError>;

    /// Mutes or unmutes the device.
    ///
    /// # Errors
    ///
    /// Returns an error if the mute state cannot be changed.
    fn set_mute(&self, muted: bool) -> Result<(), AudioError>;

    /// Returns the unique identifier for this device.
    ///
    /// The returned value is the same opaque string that [`Self::list`] yields
    /// as the first element of each `(id, name)` pair and that
    /// [`Self::from_id`] accepts as its argument.
    ///
    /// The value is guaranteed to be non-empty.
    ///
    /// # Platform-specific formats
    ///
    /// | Platform | Format                                                  |
    /// |----------|---------------------------------------------------------|
    /// | Linux    | PulseAudio sink name (e.g. `alsa_output.pci-0000_…`)    |
    /// | Windows  | WASAPI endpoint ID (e.g. `{0.0.0.00000000}.{…}`)       |
    /// | macOS    | CoreAudio device UID (numeric string, e.g. `"73"`)      |
    fn id(&self) -> &str;

    /// Returns the human-readable display name of this device.
    ///
    /// The returned value is the same string that [`Self::list`] yields as the
    /// second element of each `(id, name)` pair and that [`Self::from_name`]
    /// uses for substring matching.
    ///
    /// The value is guaranteed to be non-empty.
    ///
    /// # Platform-specific formats
    ///
    /// | Platform | Format                                                  |
    /// |----------|---------------------------------------------------------|
    /// | Linux    | PulseAudio sink description (e.g. `"Built-in Audio"`)   |
    /// | Windows  | WASAPI endpoint friendly name (e.g. `"Speakers"`)       |
    /// | macOS    | CoreAudio device name (e.g. `"MacBook Pro Speakers"`)   |
    fn name(&self) -> &str;
}
