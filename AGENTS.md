# AGENTS.md — AI agent instructions for the `volumecontrol` workspace

This document describes the conventions, structure, and rules that AI coding
agents should follow when contributing to this repository.

---

## Repository structure

```
volumecontrol/                   ← workspace root
├── Cargo.toml                   ← workspace manifest
├── AGENTS.md                    ← this file
├── volumecontrol-core/          ← platform-independent traits & errors
├── volumecontrol-linux/         ← PulseAudio (libpulse-binding) backend
├── volumecontrol-windows/       ← WASAPI (windows crate) backend
├── volumecontrol-macos/         ← CoreAudio (objc2-core-audio) backend
└── volumecontrol/               ← cross-platform wrapper (re-exports the
                                    correct backend for the current target)
```

---

## Crate roles

| Crate                   | Purpose                                                        |
|-------------------------|----------------------------------------------------------------|
| `volumecontrol-core`    | `AudioDevice` trait, `AudioError` enum, shared utilities       |
| `volumecontrol-linux`   | `AudioDevice` impl using PulseAudio; requires `pulseaudio` feature |
| `volumecontrol-windows` | `AudioDevice` impl using WASAPI; requires `wasapi` feature     |
| `volumecontrol-macos`   | `AudioDevice` impl using CoreAudio; requires `coreaudio` feature |
| `volumecontrol`         | Selects the right backend at compile time via `#[cfg(target_os)]` |

---

## Public API

Every platform crate exposes an `AudioDevice` struct that implements
`volumecontrol_core::AudioDevice`.  The methods and their signatures are:

```rust
pub trait AudioDevice: Sized {
    fn from_default()             -> Result<Self, AudioError>;
    fn from_id(id: &str)      -> Result<Self, AudioError>;
    fn from_name(name: &str)  -> Result<Self, AudioError>;
    fn list()                 -> Result<Vec<(String, String)>, AudioError>;
    fn get_vol(&self)         -> Result<u8, AudioError>;
    fn set_vol(&self, vol: u8)-> Result<(), AudioError>;
    fn is_mute(&self)         -> Result<bool, AudioError>;
    fn set_mute(&self, muted: bool) -> Result<(), AudioError>;
    fn id(&self)              -> &str;
    fn name(&self)            -> &str;
}
```

`list()` returns `(id, name)` pairs.  Volume is always in the range `0..=100`.

`id()` returns the platform-specific unique identifier for the device — the same
string that `list()` surfaces as the first element of each pair and that
`from_id()` accepts.  The value is never empty.

`name()` returns the human-readable display name for the device — the same
string that `list()` surfaces as the second element of each pair and that
`from_name()` uses for substring matching.  The value is never empty.

Platform-specific identifier formats:

| Platform | `id()` format                                      | `name()` format              |
|----------|----------------------------------------------------|------------------------------|
| Linux    | PulseAudio sink name (e.g. `alsa_output.pci-…`)    | PulseAudio sink description  |
| Windows  | WASAPI endpoint ID (e.g. `{0.0.0.00000000}.{…}`)  | WASAPI endpoint friendly name |
| macOS    | CoreAudio device UID (numeric string, e.g. `"73"`) | CoreAudio device name        |

---

## Hard rules — must be followed at all times

1. **Never use `unwrap()` or `expect()`.**  Always propagate errors with `?`
   or return a typed `Result`.
2. **Never expose `unsafe` in the public API.**  All unsafe code must be
   encapsulated inside a private helper and documented with a `// SAFETY:`
   comment explaining why it is sound.
3. **Use `thiserror` for all error types.**  Every `enum` representing errors
   must derive `thiserror::Error`.
4. **Respect all Clippy lints.**  Run `cargo clippy --all-targets --all-features`
   and fix every warning before marking work as done.
5. **Write tests for every public method.**  Tests live in a
   `#[cfg(test)] mod tests { … }` block inside the same file.
6. **Keep the build green at all times.**  Do not commit code that fails
   `cargo check`, `cargo test`, or `cargo clippy`.

---

## Adding a new dependency

1. Check the [GitHub Advisory Database](https://github.com/advisories) for
   known vulnerabilities **before** adding any dependency.
2. Prefer well-maintained crates with recent releases.
3. Declare shared dependencies in `[workspace.dependencies]` and reference
   them with `{ workspace = true }` in individual crates.
4. Platform-specific native libraries (libpulse, windows, objc2-core-audio)
   must be **optional** features; the crate must compile without them.

---

## Platform-specific implementation workflow

Each platform crate uses feature flags to gate the real implementation:

```
pulseaudio  → enables libpulse-binding  (volumecontrol-linux)
wasapi      → enables windows crate     (volumecontrol-windows)
coreaudio   → enables objc2-core-audio  (volumecontrol-macos)
```

When no feature is enabled every method returns `Err(AudioError::Unsupported)`.

To implement a backend:
1. Enable the feature: `cargo build -p volumecontrol-linux --features pulseaudio`
2. Replace the `todo!()` stubs with real PulseAudio / WASAPI / CoreAudio calls.
3. Keep all unsafe code hidden behind safe wrappers.
4. Ensure tests pass: `cargo test -p volumecontrol-linux --features pulseaudio`

---

## Running the tools

```bash
# Check the whole workspace
cargo check --workspace

# Run all tests
cargo test --workspace

# Lint with Clippy (no warnings allowed)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format code
cargo fmt --all
```

---

## Coding style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Use `rustfmt` defaults (enforced by `cargo fmt`).
- Prefer `snake_case` for files and modules.
- Document every public item with a doc comment (`///`).
- Add `# Errors` sections to doc comments for fallible functions.
- Add `# Safety` sections to doc comments for `unsafe fn`.
- Crate-level documentation goes in `src/lib.rs` as `//!` comments.
