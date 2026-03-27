# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Cross-platform `AudioDevice` API supporting Linux (PulseAudio), Windows (WASAPI), and macOS (CoreAudio).
- `AudioDevice::from_default()` — open the system default audio output device.
- `AudioDevice::from_id(id)` — look up a device by its platform-specific identifier.
- `AudioDevice::from_name(name)` — find a device by case-insensitive substring match on its name.
- `AudioDevice::list()` — enumerate all available audio output devices as `Vec<DeviceInfo>`.
- `AudioDevice::get_vol()` / `set_vol(u8)` — read and write volume in the range `0..=100`.
- `AudioDevice::is_mute()` / `set_mute(bool)` — read and toggle the mute state.
- `AudioDevice::id()` / `name()` — access the device's unique identifier and human-readable name.
- `AudioError` enum with variants: `DeviceNotFound`, `InitializationFailed`, `ListFailed`, `GetVolumeFailed`, `SetVolumeFailed`, `GetMuteFailed`, `SetMuteFailed`, `Unsupported`, `EndpointLockPoisoned`.
- `DeviceInfo` struct implementing `Display` as `"name (id)"`.
- `volumecontrol-core` crate with the `AudioDevice` trait and shared error/data types.
- `volumecontrol-linux` crate with PulseAudio backend (`pulseaudio` feature).
- `volumecontrol-windows` crate with WASAPI backend (`wasapi` feature).
- `volumecontrol-macos` crate with CoreAudio backend (`coreaudio` feature).
- `volumecontrol` wrapper crate that auto-selects the correct backend at compile time.

[Unreleased]: https://github.com/SomaticIT/volumecontrol/compare/HEAD...HEAD
