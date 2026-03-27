//! Core traits, errors, and shared types for the `volumecontrol` crate family.
//!
//! This crate defines the [`AudioDevice`] trait and the [`AudioError`] and
//! [`DeviceInfo`] types that are shared across all platform backends.  It is
//! not intended to be used directly; instead, depend on the
//! [`volumecontrol`](https://crates.io/crates/volumecontrol) crate, which
//! selects the right backend automatically.
//!
//! # Example
//!
//! ```
//! use volumecontrol_core::AudioError;
//!
//! // `AudioError` is returned by all fallible operations across backends.
//! let err = AudioError::DeviceNotFound;
//! assert_eq!(err.to_string(), "audio device not found");
//! ```

#![deny(missing_docs)]

mod error;
mod structs;
mod traits;

pub use error::AudioError;
pub use structs::DeviceInfo;
pub use traits::AudioDevice;
