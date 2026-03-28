# volumecontrol-windows

> Windows (WASAPI) volume control backend for the `volumecontrol` crate family.

This crate provides an `AudioDevice` implementation backed by WASAPI.

> **Note:** This crate exists primarily as an implementation detail of the
> [`volumecontrol`](https://crates.io/crates/volumecontrol) crate, which
> selects the correct backend automatically.  If cross-platform support is not
> a concern, you may depend on this crate directly.

---

## Feature flags

| Feature  | Description                                       | Requires          |
|----------|---------------------------------------------------|-------------------|
| `wasapi` | Enable the real WASAPI backend via `windows`      | Windows target only |

Without the `wasapi` feature every method returns `AudioError::Unsupported`,
which allows the crate to compile on any platform without the Windows SDK.

## Usage

```toml
[dependencies]
volumecontrol-windows = { version = "0.1", features = ["wasapi"] }
volumecontrol-core    = "0.1"
```

## Example

```no_run
use volumecontrol_windows::AudioDevice;
use volumecontrol_core::AudioDevice as _;

fn main() -> Result<(), volumecontrol_core::AudioError> {
    let device = AudioDevice::from_default()?;
    println!("{device}");  // e.g. "Speakers ({0.0.0.00000000}.{…})"
    println!("Current volume: {}%", device.get_vol()?);
    Ok(())
}
```

## Built with AI

This crate is part of the `volumecontrol` workspace, which was built **100% with [GitHub Copilot](https://github.com/features/copilot)** (Claude Opus & Claude Sonnet) as an experiment in AI-driven development of a production-ready Rust crate.

## License

MIT — see the LICENSE file for details.
