# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/touchifyapp/volumecontrol/compare/volumecontrol-v0.1.2...volumecontrol-v0.1.3) - 2026-04-01

### Other

- add npm and other badges to README files ([#73](https://github.com/touchifyapp/volumecontrol/pull/73))

## [0.1.2](https://github.com/touchifyapp/volumecontrol/compare/volumecontrol-v0.1.1...volumecontrol-v0.1.2) - 2026-04-01

### Added

- *(napi)* add volumecontrol-napi crate with Node.js bindings via napi-rs ([#54](https://github.com/touchifyapp/volumecontrol/pull/54))

## [0.1.1](https://github.com/touchifyapp/volumecontrol/compare/volumecontrol-v0.1.0...volumecontrol-v0.1.1) - 2026-03-28

### Other

- add "Built with AI" notice to all README files ([#48](https://github.com/touchifyapp/volumecontrol/pull/48))
- update stale repo links from SomaticIT to touchifyapp ([#46](https://github.com/touchifyapp/volumecontrol/pull/46))
- release v0.1.0 ([#43](https://github.com/touchifyapp/volumecontrol/pull/43))

## [0.1.0](https://github.com/touchifyapp/volumecontrol/releases/tag/volumecontrol-v0.1.0) - 2026-03-28

### Added

- implement `Display` for `DeviceInfo` and all `AudioDevice` backends; enforce via supertrait ([#37](https://github.com/touchifyapp/volumecontrol/pull/37))
- replace `Vec<(String, String)>` in `list()` with `DeviceInfo` struct ([#29](https://github.com/touchifyapp/volumecontrol/pull/29))
- add `id()` and `name()` getters to `AudioDevice` trait across all platforms ([#15](https://github.com/touchifyapp/volumecontrol/pull/15))
- implement the `volumecontrol` wrapper crate for unified usage and testing ([#11](https://github.com/touchifyapp/volumecontrol/pull/11))
- create volumecontrol Cargo workspace with all platform crates

### Fixed

- remove `as AudioDeviceTrait` alias pattern ([#13](https://github.com/touchifyapp/volumecontrol/pull/13))

### Other

- add release-plz automated release pipeline ([#42](https://github.com/touchifyapp/volumecontrol/pull/42))
- prepare documentation and metadata for crates.io publication ([#39](https://github.com/touchifyapp/volumecontrol/pull/39))
- surface id() and name() accessors across AGENTS.md, trait, wrapper, and README ([#21](https://github.com/touchifyapp/volumecontrol/pull/21))
- rename `default()` to `from_default()` across all crates ([#19](https://github.com/touchifyapp/volumecontrol/pull/19))
- add comprehensive README.md ([#17](https://github.com/touchifyapp/volumecontrol/pull/17))
- Initial commit
