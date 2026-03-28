# volumecontrol-linux

> Linux (PulseAudio) volume control backend for the `volumecontrol` crate family.

This crate provides an `AudioDevice` implementation backed by PulseAudio.

> **Note:** This crate exists primarily as an implementation detail of the
> [`volumecontrol`](https://crates.io/crates/volumecontrol) crate, which
> selects the correct backend automatically.  If cross-platform support is not
> a concern, you may depend on this crate directly.

---

## Feature flags

| Feature      | Description                                                | Requires               |
|--------------|------------------------------------------------------------|------------------------|
| `pulseaudio` | Enable the real PulseAudio backend via `libpulse-binding`  | `libpulse-dev` package |

Without the `pulseaudio` feature every method returns `AudioError::Unsupported`,
which allows the crate to compile on any platform without the PulseAudio
development headers.

## System requirements

On Linux, install the PulseAudio development headers before building:

```bash
# Debian / Ubuntu
sudo apt-get install libpulse-dev

# Fedora / RHEL
sudo dnf install pulseaudio-libs-devel

# Arch Linux
sudo pacman -S libpulse
```

## Usage

```toml
[dependencies]
volumecontrol-linux = { version = "0.1", features = ["pulseaudio"] }
volumecontrol-core  = "0.1"
```

## Example

```no_run
use volumecontrol_linux::AudioDevice;
use volumecontrol_core::AudioDevice as _;

fn main() -> Result<(), volumecontrol_core::AudioError> {
    let device = AudioDevice::from_default()?;
    println!("{device}");  // e.g. "Built-in Audio (alsa_output.pci-…)"
    println!("Current volume: {}%", device.get_vol()?);
    Ok(())
}
```

## Built with AI

This crate is part of the `volumecontrol` workspace, which was built **100% with [GitHub Copilot](https://github.com/features/copilot)** (Claude Opus & Claude Sonnet) as an experiment in AI-driven development of a production-ready Rust crate.

## License

MIT — see the LICENSE file in the repository for details.
