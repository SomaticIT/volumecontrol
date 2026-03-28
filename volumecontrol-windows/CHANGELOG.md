# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/touchifyapp/volumecontrol/releases/tag/volumecontrol-windows-v0.1.0) - 2026-03-28

### Added

- implement `Display` for `DeviceInfo` and all `AudioDevice` backends; enforce via supertrait ([#37](https://github.com/touchifyapp/volumecontrol/pull/37))
- replace `Vec<(String, String)>` in `list()` with `DeviceInfo` struct ([#29](https://github.com/touchifyapp/volumecontrol/pull/29))
- add `id()` and `name()` getters to `AudioDevice` trait across all platforms ([#15](https://github.com/touchifyapp/volumecontrol/pull/15))
- implement WASAPI backend for volumecontrol-windows with internal module organization and Windows CI ([#6](https://github.com/touchifyapp/volumecontrol/pull/6))
- create volumecontrol Cargo workspace with all platform crates

### Fixed

- propagate WASAPI mutex poisoning as `AudioError::EndpointLockPoisoned` ([#31](https://github.com/touchifyapp/volumecontrol/pull/31))

### Other

- add release-plz automated release pipeline ([#42](https://github.com/touchifyapp/volumecontrol/pull/42))
- prepare documentation and metadata for crates.io publication ([#39](https://github.com/touchifyapp/volumecontrol/pull/39))
- normalize `from_name()` to case-insensitive substring matching across all backends ([#28](https://github.com/touchifyapp/volumecontrol/pull/28))
- windows backend: cache IAudioEndpointVolume per device ([#27](https://github.com/touchifyapp/volumecontrol/pull/27))
- rename `default()` to `from_default()` across all crates ([#19](https://github.com/touchifyapp/volumecontrol/pull/19))
