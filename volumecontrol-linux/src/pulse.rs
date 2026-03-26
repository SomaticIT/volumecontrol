//! Internal PulseAudio helpers for `volumecontrol-linux`.
//!
//! This module exposes a [`PulseConnection`] struct that holds a cached
//! `Mainloop` + `Context` pair.  Callers should reuse a single
//! [`PulseConnection`] across multiple operations to avoid the overhead of
//! re-establishing a PulseAudio connection on every call.
//!
//! [`PulseConnection`] is `!Send` because the underlying [`Mainloop`] and
//! [`Context`] types from `libpulse-binding` are `!Send`.  Use on a single
//! thread only.

use std::{cell::RefCell, mem::ManuallyDrop, rc::Rc};

use libpulse_binding as pulse;
use pulse::{
    callbacks::ListResult,
    context::{Context, FlagSet as ContextFlagSet, State as ContextState},
    mainloop::standard::{IterateResult, Mainloop},
    operation::State as OperationState,
    volume::{ChannelVolumes, Volume},
};

use volumecontrol_core::AudioError;

// ─── Data types ─────────────────────────────────────────────────────────────

/// A snapshot of a PulseAudio sink's runtime state.
#[derive(Clone)]
pub(crate) struct SinkSnapshot {
    /// PulseAudio sink name (used as the device identifier).
    pub(crate) name: String,
    /// Human-readable sink description.
    pub(crate) description: String,
    /// Current volume level, `0..=100`.
    pub(crate) volume: u8,
    /// Whether the sink is currently muted.
    pub(crate) mute: bool,
}

// ─── Volume helpers ──────────────────────────────────────────────────────────

/// Converts a PulseAudio [`Volume`] value to a percentage in `0..=100`.
fn volume_to_pct(v: Volume) -> u8 {
    let norm = Volume::NORMAL.0 as f64;
    if norm == 0.0 {
        return 0;
    }
    ((v.0 as f64 / norm) * 100.0).round().clamp(0.0, 100.0) as u8
}

/// Converts a percentage `0..=100` to a PulseAudio [`Volume`] value.
fn pct_to_volume(pct: u8) -> Volume {
    let norm = Volume::NORMAL.0 as f64;
    Volume(((pct.min(100) as f64 / 100.0) * norm).round() as u32)
}

// ─── Connection / loop helpers ───────────────────────────────────────────────

/// Opens a PulseAudio standard main loop and connects a context to the server.
///
/// Returns a ready-to-use `(Mainloop, Context)` pair, or an
/// [`AudioError::InitializationFailed`] if the connection cannot be
/// established.
fn connect() -> Result<(Mainloop, Context), AudioError> {
    let mut mainloop = Mainloop::new().ok_or_else(|| {
        AudioError::InitializationFailed("could not create PulseAudio main loop".into())
    })?;

    let mut context = Context::new(&mainloop, "volumecontrol").ok_or_else(|| {
        AudioError::InitializationFailed("could not create PulseAudio context".into())
    })?;

    context
        .connect(None, ContextFlagSet::NOFLAGS, None)
        .map_err(|e| {
            AudioError::InitializationFailed(format!("PulseAudio connect error: {e:?}"))
        })?;

    // Pump the loop until the context reaches Ready (or a terminal state).
    loop {
        match mainloop.iterate(false) {
            IterateResult::Quit(_) => {
                return Err(AudioError::InitializationFailed(
                    "PulseAudio main loop quit during connect".into(),
                ))
            }
            IterateResult::Err(e) => {
                return Err(AudioError::InitializationFailed(format!(
                    "PulseAudio main loop error during connect: {e:?}"
                )))
            }
            IterateResult::Success(_) => {}
        }
        match context.get_state() {
            ContextState::Ready => break,
            ContextState::Failed | ContextState::Terminated => {
                return Err(AudioError::InitializationFailed(
                    "PulseAudio context failed to connect to server".into(),
                ))
            }
            _ => {}
        }
    }

    Ok((mainloop, context))
}

/// Pumps `mainloop` until `op` transitions out of the `Running` state.
///
/// Returns `Ok(())` when the operation is `Done`, or an error if the main loop
/// encounters a fatal condition or the operation is cancelled.
fn wait_for_op<C: ?Sized>(
    mainloop: &mut Mainloop,
    op: &pulse::operation::Operation<C>,
) -> Result<(), AudioError> {
    loop {
        match mainloop.iterate(false) {
            IterateResult::Quit(_) => {
                return Err(AudioError::InitializationFailed(
                    "PulseAudio main loop quit unexpectedly".into(),
                ))
            }
            IterateResult::Err(e) => {
                return Err(AudioError::InitializationFailed(format!(
                    "PulseAudio main loop error: {e:?}"
                )))
            }
            IterateResult::Success(_) => {}
        }
        match op.get_state() {
            OperationState::Done => return Ok(()),
            OperationState::Cancelled => {
                return Err(AudioError::InitializationFailed(
                    "PulseAudio operation was cancelled".into(),
                ))
            }
            OperationState::Running => {}
        }
    }
}

// ─── Cached connection ───────────────────────────────────────────────────────

/// A cached PulseAudio connection: a standard main loop paired with a context.
///
/// Reuse a single `PulseConnection` across multiple operations to avoid the
/// overhead of re-establishing a connection on every call.
///
/// # Drop order
///
/// PulseAudio's standard main loop holds deferred events that are also
/// referenced by the context.  The context **must** be dropped before the
/// main loop; otherwise the main loop frees its deferred events first and the
/// context's subsequent cleanup fires `Assertion '!e->dead' failed` inside
/// `mainloop_defer_free()`.  [`ManuallyDrop`] lets us enforce this order in
/// the [`Drop`] impl regardless of struct field declaration order.
///
/// # Thread safety
///
/// `PulseConnection` is `!Send` because [`Mainloop`] and [`Context`] from
/// `libpulse-binding` are `!Send`.  Use on a single thread only.
pub(crate) struct PulseConnection {
    mainloop: ManuallyDrop<Mainloop>,
    context: ManuallyDrop<Context>,
}

impl Drop for PulseConnection {
    fn drop(&mut self) {
        // SAFETY: both fields are valid and non-null — they were set in `new()`
        // or `ensure_ready()` and are never taken out of `ManuallyDrop`
        // elsewhere.  We drop `context` first so that PulseAudio's internal
        // deferred-event list is cleared before the mainloop tears down its own
        // event infrastructure.
        unsafe {
            ManuallyDrop::drop(&mut self.context);
            ManuallyDrop::drop(&mut self.mainloop);
        }
    }
}

impl PulseConnection {
    /// Opens a new connection to the PulseAudio server.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::InitializationFailed`] if the connection cannot
    /// be established.
    pub(crate) fn new() -> Result<Self, AudioError> {
        let (mainloop, context) = connect()?;
        Ok(Self {
            mainloop: ManuallyDrop::new(mainloop),
            context: ManuallyDrop::new(context),
        })
    }

    /// Reconnects to the PulseAudio server if the context is no longer in the
    /// `Ready` state (e.g. after a server disconnect).
    fn ensure_ready(&mut self) -> Result<(), AudioError> {
        if !matches!(self.context.get_state(), ContextState::Ready) {
            let (mainloop, context) = connect()?;
            // Drop the old context before the old mainloop so that PA's
            // internal reference counting sees the context gone first.
            // SAFETY: both fields were initialised in `new()` or a previous
            // `ensure_ready()` call and have not been moved out since.
            unsafe {
                ManuallyDrop::drop(&mut self.context);
                ManuallyDrop::drop(&mut self.mainloop);
            }
            self.context = ManuallyDrop::new(context);
            self.mainloop = ManuallyDrop::new(mainloop);
        }
        Ok(())
    }

    /// Returns the name of the system default PulseAudio sink, or
    /// [`AudioError::DeviceNotFound`] if no default sink is configured.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::InitializationFailed`] if the connection to the
    /// PulseAudio server fails.
    pub(crate) fn default_sink_name(&mut self) -> Result<String, AudioError> {
        self.ensure_ready()?;

        let result: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let result_cb = Rc::clone(&result);

        let op = self.context.introspect().get_server_info(move |info| {
            *result_cb.borrow_mut() = info.default_sink_name.as_deref().map(String::from);
        });
        wait_for_op(&mut self.mainloop, &op)?;

        let borrowed = result.borrow();
        let name = borrowed.clone().ok_or(AudioError::DeviceNotFound)?;
        Ok(name)
    }

    /// Returns a [`SinkSnapshot`] for the PulseAudio sink with the given name
    /// (i.e. the PA sink identifier, **not** the human-readable description).
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::DeviceNotFound`] if no sink with that name exists.
    pub(crate) fn sink_by_name(&mut self, name: &str) -> Result<SinkSnapshot, AudioError> {
        self.ensure_ready()?;

        let result: Rc<RefCell<Option<SinkSnapshot>>> = Rc::new(RefCell::new(None));
        let result_cb = Rc::clone(&result);

        let op = self
            .context
            .introspect()
            .get_sink_info_by_name(name, move |list| {
                if let ListResult::Item(info) = list {
                    *result_cb.borrow_mut() = Some(SinkSnapshot {
                        name: opt_cow_str(info.name.as_ref()),
                        description: opt_cow_str(info.description.as_ref()),
                        volume: volume_to_pct(info.volume.avg()),
                        mute: info.mute,
                    });
                }
            });
        wait_for_op(&mut self.mainloop, &op)?;

        let snap = result.borrow().clone().ok_or(AudioError::DeviceNotFound)?;
        Ok(snap)
    }

    /// Returns the first sink whose description contains `query`
    /// (case-insensitive substring match).
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::DeviceNotFound`] if no matching sink is found.
    pub(crate) fn sink_matching_description(
        &mut self,
        query: &str,
    ) -> Result<SinkSnapshot, AudioError> {
        let query_lower = query.to_lowercase();
        self.list_sink_snapshots()?
            .into_iter()
            .find(|s| s.description.to_lowercase().contains(&query_lower))
            .ok_or(AudioError::DeviceNotFound)
    }

    /// Lists all PulseAudio sinks as `(name, description)` pairs.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::InitializationFailed`] if the enumeration fails.
    pub(crate) fn list_sinks(&mut self) -> Result<Vec<(String, String)>, AudioError> {
        Ok(self
            .list_sink_snapshots()?
            .into_iter()
            .map(|s| (s.name, s.description))
            .collect())
    }

    /// Sets the volume of the named sink to `vol` (`0..=100`), preserving the
    /// channel layout of the sink.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::SetVolumeFailed`] if the server rejects the change.
    pub(crate) fn set_sink_volume(&mut self, name: &str, vol: u8) -> Result<(), AudioError> {
        self.ensure_ready()?;

        // Fetch the current channel-volume to preserve the channel layout.
        let cv: Rc<RefCell<Option<ChannelVolumes>>> = Rc::new(RefCell::new(None));
        let cv_cb = Rc::clone(&cv);

        let op = self
            .context
            .introspect()
            .get_sink_info_by_name(name, move |list| {
                if let ListResult::Item(info) = list {
                    *cv_cb.borrow_mut() = Some(info.volume);
                }
            });
        wait_for_op(&mut self.mainloop, &op)?;

        let cv_opt: Option<ChannelVolumes> = *cv.borrow();
        let mut volumes = cv_opt.ok_or(AudioError::DeviceNotFound)?;

        let pa_vol = pct_to_volume(vol);
        volumes.set(volumes.len(), pa_vol);

        let success: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let success_cb = Rc::clone(&success);

        let mut insp = self.context.introspect();
        let op2 = insp.set_sink_volume_by_name(
            name,
            &volumes,
            Some(Box::new(move |ok| *success_cb.borrow_mut() = ok)),
        );
        wait_for_op(&mut self.mainloop, &op2)?;

        if *success.borrow() {
            Ok(())
        } else {
            Err(AudioError::SetVolumeFailed(
                "PulseAudio server rejected the volume change".into(),
            ))
        }
    }

    /// Sets the mute state of the named sink.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::SetMuteFailed`] if the server rejects the change.
    pub(crate) fn set_sink_mute(&mut self, name: &str, muted: bool) -> Result<(), AudioError> {
        self.ensure_ready()?;

        let success: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let success_cb = Rc::clone(&success);

        let mut insp = self.context.introspect();
        let op = insp.set_sink_mute_by_name(
            name,
            muted,
            Some(Box::new(move |ok| *success_cb.borrow_mut() = ok)),
        );
        wait_for_op(&mut self.mainloop, &op)?;

        if *success.borrow() {
            Ok(())
        } else {
            Err(AudioError::SetMuteFailed(
                "PulseAudio server rejected the mute state change".into(),
            ))
        }
    }

    // ─── Private helpers ─────────────────────────────────────────────────────

    /// Enumerates all PulseAudio sinks and returns a `Vec<SinkSnapshot>`.
    fn list_sink_snapshots(&mut self) -> Result<Vec<SinkSnapshot>, AudioError> {
        self.ensure_ready()?;

        let result: Rc<RefCell<Vec<SinkSnapshot>>> = Rc::new(RefCell::new(Vec::new()));
        let result_cb = Rc::clone(&result);

        let op = self.context.introspect().get_sink_info_list(move |list| {
            if let ListResult::Item(info) = list {
                result_cb.borrow_mut().push(SinkSnapshot {
                    name: opt_cow_str(info.name.as_ref()),
                    description: opt_cow_str(info.description.as_ref()),
                    volume: volume_to_pct(info.volume.avg()),
                    mute: info.mute,
                });
            }
        });
        wait_for_op(&mut self.mainloop, &op)?;

        let sinks = result.borrow().clone();
        Ok(sinks)
    }
}

/// Converts an `Option<Cow<str>>` reference to an owned `String`, returning
/// an empty string when the option is `None`.
fn opt_cow_str(s: Option<&std::borrow::Cow<'_, str>>) -> String {
    s.map(|c| c.as_ref()).unwrap_or("").to_owned()
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_to_pct_normal_is_100() {
        assert_eq!(volume_to_pct(Volume::NORMAL), 100);
    }

    #[test]
    fn volume_to_pct_muted_is_0() {
        assert_eq!(volume_to_pct(Volume::MUTED), 0);
    }

    #[test]
    fn pct_to_volume_100_is_normal() {
        assert_eq!(pct_to_volume(100), Volume::NORMAL);
    }

    #[test]
    fn pct_to_volume_0_is_muted() {
        assert_eq!(pct_to_volume(0), Volume::MUTED);
    }

    #[test]
    fn round_trip_volume_pct() {
        for pct in [0u8, 25, 50, 75, 100] {
            let recovered = volume_to_pct(pct_to_volume(pct));
            assert_eq!(recovered, pct, "round-trip failed for {pct}%");
        }
    }

    #[test]
    fn pct_to_volume_clamps_above_100() {
        // u8 max is 255; pct_to_volume should clamp at 100.
        assert_eq!(pct_to_volume(200_u8), pct_to_volume(100));
    }
}
