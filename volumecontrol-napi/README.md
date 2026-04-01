# volumecontrol-napi

[![npm](https://img.shields.io/npm/v/@touchifyapp/volumecontrol.svg)](https://www.npmjs.com/package/@touchifyapp/volumecontrol)
[![CI](https://github.com/touchifyapp/volumecontrol/actions/workflows/ci.yml/badge.svg)](https://github.com/touchifyapp/volumecontrol/actions)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/touchifyapp/volumecontrol/blob/main/LICENSE)

> Node.js bindings for the `volumecontrol` crate family via napi-rs.

This package exposes the cross-platform audio volume control API to Node.js as a native addon built with [napi-rs](https://napi.rs). The correct system backend (PulseAudio on Linux, WASAPI on Windows, CoreAudio on macOS) is selected automatically at compile time.

---

## Installation

```bash
npm install @touchifyapp/volumecontrol
```

> **Platform requirements**
> - Node.js >= 18
> - Linux: `libpulse-dev` (PulseAudio development headers)
> - Windows / macOS: no extra system libraries required

---

## Usage

```js
const { AudioDevice } = require('@touchifyapp/volumecontrol');

// Open the default audio output device
const device = AudioDevice.fromDefault();
console.log(`${device.name} (${device.id})`);
console.log(`Volume: ${device.getVol()}%`);

// Change volume (0–100)
device.setVol(50);

// Check mute state
console.log(`Muted: ${device.isMute()}`);

// Mute / unmute the device
device.setMute(true);   // mute
device.setMute(false);  // unmute

// List all available audio devices
const devices = AudioDevice.list();
devices.forEach(d => console.log(`${d.name} (${d.id})`));
```

Rust `snake_case` method names are automatically mapped to JavaScript `camelCase` by napi-rs:

| Rust              | JavaScript          |
|-------------------|---------------------|
| `from_default()`  | `fromDefault()`     |
| `from_id(id)`     | `fromId(id)`        |
| `from_name(name)` | `fromName(name)`    |
| `get_vol()`       | `getVol()`          |
| `set_vol(vol)`    | `setVol(vol)`       |
| `is_mute()`       | `isMute()`          |
| `set_mute(muted)` | `setMute(muted)`    |
| `list()`          | `list()`            |

---

## Building from source

Prerequisites: [Rust](https://rustup.rs/) toolchain and the platform system libraries listed above.

```bash
cd volumecontrol-napi
npm install
npm run build       # release build
npm run build:debug # debug build
```

The build produces a `.node` native addon file and an auto-generated `index.d.ts` TypeScript declaration file.

---

## Built with AI

This crate is part of the `volumecontrol` workspace, which was built **100% with [GitHub Copilot](https://github.com/features/copilot)** (Claude Opus & Claude Sonnet) as an experiment in AI-driven development of a production-ready Rust crate.

---

## License

MIT — see the LICENSE file in the repository for details.
