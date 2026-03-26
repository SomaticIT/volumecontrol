/// Metadata for an available audio device returned by [`crate::AudioDevice::list`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// Unique platform-specific device identifier.
    ///
    /// This is the same opaque string that [`crate::AudioDevice::from_id`]
    /// accepts as its argument.  It is guaranteed to be non-empty.
    pub id: String,
    /// Human-readable device name.
    ///
    /// This is the same string that [`crate::AudioDevice::from_name`] uses for
    /// substring matching.  It is guaranteed to be non-empty.
    pub name: String,
}
