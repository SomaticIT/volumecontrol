# volumecontrol-core

> Core traits, errors, and shared types for the `volumecontrol` crate family.

This crate defines the `AudioDevice` trait and the `AudioError` and
`DeviceInfo` types that are shared across all platform backends.

> **Note:** This crate is not intended to be used directly.  Depend on the
> [`volumecontrol`](https://crates.io/crates/volumecontrol) crate, which
> selects the right platform backend automatically.

---

## Usage

If you are building a custom backend or need direct access to the shared types:

```toml
[dependencies]
volumecontrol-core = "0.1"
```

## Example

```rust
use volumecontrol_core::AudioError;

// `AudioError` is returned by all fallible operations across backends.
let err = AudioError::DeviceNotFound;
println!("{err}");  // "audio device not found"
```

## License

MIT — see the [LICENSE](../LICENSE) file for details.
