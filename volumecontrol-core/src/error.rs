use thiserror::Error;

/// Error type for all volumecontrol operations.
#[derive(Debug, Error)]
pub enum AudioError {
    /// The requested device was not found.
    #[error("audio device not found")]
    DeviceNotFound,

    /// The audio subsystem could not be initialized.
    #[error("failed to initialize audio subsystem: {0}")]
    InitializationFailed(String),

    /// Listing available devices failed.
    #[error("failed to list audio devices: {0}")]
    ListFailed(String),

    /// Reading the current volume level failed.
    #[error("failed to retrieve volume: {0}")]
    GetVolumeFailed(String),

    /// Changing the volume level failed.
    #[error("failed to set volume: {0}")]
    SetVolumeFailed(String),

    /// Reading the mute state failed.
    #[error("failed to retrieve mute state: {0}")]
    GetMuteFailed(String),

    /// Changing the mute state failed.
    #[error("failed to set mute state: {0}")]
    SetMuteFailed(String),

    /// The operation is not supported on the current platform.
    #[error("operation not supported on this platform")]
    Unsupported,

    /// A mutex protecting a platform audio endpoint was poisoned (a thread
    /// panicked while holding the lock).  The endpoint state may be
    /// inconsistent; callers should discard the device and re-acquire it.
    #[error("audio endpoint mutex was poisoned")]
    EndpointLockPoisoned,
}
