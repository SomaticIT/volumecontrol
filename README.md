# volumecontrol

[![Crates.io](https://img.shields.io/crates/v/volumecontrol.svg)](https://crates.io/crates/volumecontrol)
[![docs.rs](https://docs.rs/volumecontrol/badge.svg)](https://docs.rs/volumecontrol)
[![CI](https://github.com/touchifyapp/volumecontrol/actions/workflows/ci.yml/badge.svg)](https://github.com/touchifyapp/volumecontrol/actions)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/touchifyapp/volumecontrol/blob/main/LICENSE)

> A cross-platform Rust library for querying and controlling system audio volume.

`volumecontrol` exposes a single, unified API that works on **Linux** (PulseAudio), **Windows** (WASAPI), and **macOS** (CoreAudio). The correct backend is selected automatically at compile time — no feature flags or platform-specific imports are needed in your code.

---

## Table of Contents

- [Repository Structure](#repository-structure)
- [Getting Started](#getting-started)
- [Installation](#installation)
- [Usage](#usage)
- [How to Collaborate](#how-to-collaborate)
- [License](#license)
- [Contact / Further Information](#contact--further-information)

---

## Repository Structure

```
volumecontrol/                   ← workspace root
├── Cargo.toml                   ← workspace manifest & cross-compilation config
├── Cargo.lock                   ← locked dependency versions
├── LICENSE                      ← MIT license
├── README.md                    ← this file
├── AGENTS.md                    ← AI agent contribution instructions
│
├── volumecontrol-core/          ← platform-independent traits & error types
│   └── src/
│       ├── lib.rs               ← public re-exports
│       ├── traits.rs            ← AudioDevice trait definition
│       └── error.rs             ← AudioError enum
│
├── volumecontrol-linux/         ← PulseAudio backend (feature: pulseaudio)
├── volumecontrol-windows/       ← WASAPI backend       (feature: wasapi)
├── volumecontrol-macos/         ← CoreAudio backend    (feature: coreaudio)
│
└── volumecontrol/               ← cross-platform wrapper crate
    └── src/lib.rs               ← selects the right backend via #[cfg(target_os)]
```

| Crate                   | Purpose                                                        |
|-------------------------|----------------------------------------------------------------|
| `volumecontrol-core`    | `AudioDevice` trait, `AudioError` enum, shared utilities       |
| `volumecontrol-linux`   | `AudioDevice` impl using PulseAudio                            |
| `volumecontrol-windows` | `AudioDevice` impl using WASAPI                                |
| `volumecontrol-macos`   | `AudioDevice` impl using CoreAudio                             |
| `volumecontrol`         | Selects the right backend at compile time via `#[cfg(target_os)]` |

---

## Getting Started

`volumecontrol` lets you read and change the system audio volume from Rust with a single, cross-platform API:

```rust
use volumecontrol::AudioDevice;

fn main() -> Result<(), volumecontrol::AudioError> {
    let device = AudioDevice::from_default()?;
    println!("Current volume: {}%", device.get_vol()?);
    Ok(())
}
```

### Platform requirements

| Platform | Backend    | System library required            |
|----------|------------|------------------------------------|
| Linux    | PulseAudio | `libpulse-dev` (e.g. via `apt`)    |
| Windows  | WASAPI     | built into Windows — nothing extra |
| macOS    | CoreAudio  | built into macOS — nothing extra   |

---

## Installation

Add `volumecontrol` to your `Cargo.toml`:

```toml
[dependencies]
volumecontrol = "0.1"
```

> **Linux users:** install the PulseAudio development headers before building:
>
> ```bash
> # Debian / Ubuntu
> sudo apt-get install libpulse-dev
>
> # Fedora / RHEL
> sudo dnf install pulseaudio-libs-devel
>
> # Arch Linux
> sudo pacman -S libpulse
> ```

Once the system library is in place, build as usual:

```bash
cargo build
```

---

## Usage

### Open the default audio device

```rust
use volumecontrol::AudioDevice;

let device = AudioDevice::from_default()?;
```

### Look up a device by ID or name

```rust
// By exact device identifier returned from list()
let device = AudioDevice::from_id("alsa_output.pci-0000_00_1f.3.analog-stereo")?;

// By a partial name match (case-insensitive substring search)
let device = AudioDevice::from_name("Speakers")?;
```

### List all available audio devices

```rust
let devices = AudioDevice::list()?;
for info in &devices {
    // DeviceInfo implements Display as "name (id)"
    println!("{info}");
}
```

### Read device ID and name

```rust
// Display shows "name (id)" — useful for logs and CLI output
println!("{device}");

// id() returns the opaque platform identifier used by from_id() and list()
println!("Device id:   {}", device.id());

// name() returns the human-readable label used by from_name() and list()
println!("Device name: {}", device.name());
```

Both values are guaranteed to be non-empty.

### Read and change the volume

```rust
// Volume is always in the range 0..=100
let vol = device.get_vol()?;
println!("Volume: {vol}%");

device.set_vol(50)?;   // set to 50 %
```

### Mute / unmute

```rust
if device.is_mute()? {
    println!("Device is muted");
}

device.set_mute(true)?;   // mute
device.set_mute(false)?;  // unmute
```

### Error handling

All methods return `Result<_, AudioError>`. The error variants are:

| Variant                  | Meaning                                      |
|--------------------------|----------------------------------------------|
| `DeviceNotFound`         | No device matched the given id or name       |
| `InitializationFailed`   | The audio subsystem could not be initialised |
| `ListFailed`             | Listing available devices failed             |
| `GetVolumeFailed`        | Could not read the current volume            |
| `SetVolumeFailed`        | Could not change the volume                  |
| `GetMuteFailed`          | Could not read the mute state                |
| `SetMuteFailed`          | Could not change the mute state              |
| `Unsupported`            | Operation not supported on this platform     |
| `EndpointLockPoisoned`   | A thread panicked while holding the audio endpoint lock (Windows) |

---

## How to Collaborate

Contributions are very welcome! Please follow these steps:

### 1. Open an issue first

For non-trivial changes (new features, API changes, new platform backends) open a GitHub issue to discuss the idea before investing time in a pull request.

### 2. Fork & clone

```bash
git clone https://github.com/<your-username>/volumecontrol.git
cd volumecontrol
```

### 3. Create a feature branch

```bash
git checkout -b feature/my-improvement
```

### 4. Make your changes

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Document every public item with a `///` doc comment.
- Add `# Errors` sections to fallible functions.
- **Never** use `unwrap()` or `expect()` — propagate errors with `?`.
- All `unsafe` code must be hidden behind a safe wrapper and annotated with a `// SAFETY:` comment.
- Use `thiserror` for any new error types.

### 5. Run the toolchain

```bash
# Check the whole workspace
cargo check --workspace

# Run all tests
cargo test --workspace

# Lint (no warnings allowed)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format
cargo fmt --all
```

All four commands must pass before submitting a pull request.

### 6. Submit a pull request

Push your branch and open a pull request against `main`. Describe _what_ you changed and _why_. Link the relevant issue if one exists.

### Coding style summary

| Topic              | Rule                                          |
|--------------------|-----------------------------------------------|
| Formatting         | `cargo fmt` defaults (enforced by CI)         |
| Naming             | `snake_case` for files and modules            |
| Errors             | `thiserror`-derived enums; no `unwrap`/`expect` |
| Unsafe code        | Private helpers only; `// SAFETY:` required   |
| Tests              | `#[cfg(test)] mod tests` block in the same file |
| Documentation      | `///` for every public item; `//!` for crates |

---

## Built with AI

This project was built **100% with [GitHub Copilot](https://github.com/features/copilot)** (Claude Opus & Claude Sonnet models) as an experiment in driving an AI to produce a production-ready Rust crate — from architecture and API design to platform-specific backends, CI pipelines, and documentation.

No human wrote any Rust source code directly; every implementation decision, refactor, and fix was produced by the AI under human direction.

---

## License

This project is licensed under the **MIT License**.  
See the [LICENSE](LICENSE) file for the full text.

---

## Contact / Further Information

- **Issues & feature requests:** [GitHub Issues](https://github.com/touchifyapp/volumecontrol/issues)
- **Pull requests:** [GitHub Pull Requests](https://github.com/touchifyapp/volumecontrol/pulls)
- **Maintainer:** [@SomaticIT](https://github.com/SomaticIT)

If you have a question that is not suited for a public issue, you can reach the maintainer through their GitHub profile.
