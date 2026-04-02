# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2](https://github.com/touchifyapp/volumecontrol/compare/volumecontrol-linux-v0.1.1...volumecontrol-linux-v0.1.2) - 2026-04-02

### Other

- updated the following local packages: volumecontrol-core

## [0.1.1](https://github.com/touchifyapp/volumecontrol/compare/volumecontrol-linux-v0.1.0...volumecontrol-linux-v0.1.1) - 2026-03-28

### Other

- add "Built with AI" notice to all README files ([#48](https://github.com/touchifyapp/volumecontrol/pull/48))
- release v0.1.0 ([#43](https://github.com/touchifyapp/volumecontrol/pull/43))

## [0.1.0](https://github.com/touchifyapp/volumecontrol/releases/tag/volumecontrol-linux-v0.1.0) - 2026-03-28

### Added

- implement `Display` for `DeviceInfo` and all `AudioDevice` backends; enforce via supertrait ([#37](https://github.com/touchifyapp/volumecontrol/pull/37))
- replace `Vec<(String, String)>` in `list()` with `DeviceInfo` struct ([#29](https://github.com/touchifyapp/volumecontrol/pull/29))
- add `id()` and `name()` getters to `AudioDevice` trait across all platforms ([#15](https://github.com/touchifyapp/volumecontrol/pull/15))
- implement PulseAudio backend for volumecontrol-linux ([#3](https://github.com/touchifyapp/volumecontrol/pull/3))
- create volumecontrol Cargo workspace with all platform crates

### Fixed

- add real-world PulseAudio tests and CI virtual audio environment ([#9](https://github.com/touchifyapp/volumecontrol/pull/9))

### Other

- add release-plz automated release pipeline ([#42](https://github.com/touchifyapp/volumecontrol/pull/42))
- prepare documentation and metadata for crates.io publication ([#39](https://github.com/touchifyapp/volumecontrol/pull/39))
- add standalone quality.yml workflow: fmt + doc + audit for all platforms ([#34](https://github.com/touchifyapp/volumecontrol/pull/34))
- normalize `from_name()` to case-insensitive substring matching across all backends ([#28](https://github.com/touchifyapp/volumecontrol/pull/28))
- PulseAudio backend: cache mainloop/context to avoid reconnect per call ([#25](https://github.com/touchifyapp/volumecontrol/pull/25))
- rename `default()` to `from_default()` across all crates ([#19](https://github.com/touchifyapp/volumecontrol/pull/19))
