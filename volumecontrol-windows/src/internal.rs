//! Internal WASAPI helpers for `volumecontrol-windows`.
//!
//! All `unsafe` code is confined to this module.  Every `unsafe` block carries
//! a `// SAFETY:` comment explaining why the operation is sound.
//!
//! The public-facing `AudioDevice` implementation in `lib.rs` calls only the
//! safe wrappers defined here.

#[cfg(feature = "wasapi")]
pub(crate) mod wasapi {
    use thiserror::Error;
    use volumecontrol_core::AudioError;

    use windows::Win32::{
        Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
        Media::Audio::Endpoints::IAudioEndpointVolume,
        Media::Audio::{
            eConsole, eRender, IMMDevice, IMMDeviceCollection, IMMDeviceEnumerator,
            MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
        },
        System::Com::StructuredStorage::PropVariantToStringAlloc,
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_MULTITHREADED, STGM_READ,
        },
        UI::Shell::PropertiesSystem::IPropertyStore,
    };

    // -------------------------------------------------------------------------
    // Named HRESULT constants
    // -------------------------------------------------------------------------

    /// `CoInitializeEx` result when COM was already initialised on this thread
    /// with a different apartment model.  The caller may still use COM, but
    /// must **not** call `CoUninitialize` to balance this call.
    const RPC_E_CHANGED_MODE: i32 = -2_147_417_850_i32; // 0x80010106

    /// `IMMDeviceEnumerator::GetDevice` result when no endpoint with the
    /// requested ID is registered.  Corresponds to
    /// `HRESULT_FROM_WIN32(ERROR_NOT_FOUND)`.
    const HRESULT_ERROR_NOT_FOUND: i32 = -2_147_023_216_i32; // 0x80070490

    /// `IMMDeviceEnumerator::GetDevice` result when the requested device has
    /// been removed.  Corresponds to `HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND)`.
    const HRESULT_ERROR_FILE_NOT_FOUND: i32 = -2_147_024_894_i32; // 0x80070002

    /// `IMMDeviceEnumerator::GetDevice` result for an invalidated / removed
    /// device.  Corresponds to `AUDCLNT_E_DEVICE_INVALIDATED`.
    const AUDCLNT_E_DEVICE_INVALIDATED: i32 = -2_004_287_480_i32; // 0x88890004

    // -------------------------------------------------------------------------
    // EndpointError — internal error type for cached-endpoint operations
    // -------------------------------------------------------------------------

    /// Error returned by the four endpoint-operation helpers (`get_volume`,
    /// `set_volume`, `get_mute`, `set_mute`).
    ///
    /// Splitting invalidation from every other audio error keeps the public
    /// [`AudioError`] clean: `DeviceInvalidated` is an internal implementation
    /// detail that tells [`AudioDevice::with_endpoint`] to refresh the cached
    /// [`IAudioEndpointVolume`] and retry.  It must never be converted to
    /// [`AudioError::DeviceNotFound`] until *after* a refresh attempt has also
    /// failed.
    #[derive(Debug, Error)]
    pub(crate) enum EndpointError {
        /// The WASAPI endpoint's COM interface was invalidated
        /// (`AUDCLNT_E_DEVICE_INVALIDATED`).  The caller should re-activate
        /// the interface and retry the operation.
        #[error("WASAPI endpoint was invalidated (AUDCLNT_E_DEVICE_INVALIDATED)")]
        DeviceInvalidated,
        /// Any other error returned by the endpoint operation.
        #[error(transparent)]
        Error(#[from] AudioError),
    }

    // -------------------------------------------------------------------------
    // COM lifecycle
    // -------------------------------------------------------------------------

    /// RAII guard that balances a successful [`CoInitializeEx`] call.
    ///
    /// When the guard is dropped it calls [`CoUninitialize`] **only** if this
    /// thread actually initialised COM (i.e. `CoInitializeEx` returned `S_OK`
    /// or `S_FALSE`).  If COM was already initialised in a different threading
    /// model (`RPC_E_CHANGED_MODE`) the guard does nothing on drop.
    pub(crate) struct ComGuard {
        owns_init: bool,
    }

    impl ComGuard {
        /// Initialises COM on the calling thread with the multi-threaded
        /// apartment model.
        ///
        /// # Errors
        ///
        /// Returns [`AudioError::InitializationFailed`] if COM cannot be
        /// initialised and the failure is not `RPC_E_CHANGED_MODE`.
        pub(crate) fn new() -> Result<Self, AudioError> {
            // SAFETY: CoInitializeEx is safe to call from any thread. Passing
            // `None` for the reserved parameter is explicitly documented as
            // correct.  In windows 0.62 the function returns an HRESULT value
            // directly (not wrapped in Result), so we inspect the raw code.
            let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };

            if hr.is_ok() {
                // S_OK (0x0) — first init; S_FALSE (0x1) — already init'd on
                // this thread with the same model.  Both require CoUninitialize.
                Ok(Self { owns_init: true })
            } else if hr.0 == RPC_E_CHANGED_MODE {
                // A different apartment model is active on this thread.  We can
                // still use COM but must NOT call CoUninitialize.
                Ok(Self { owns_init: false })
            } else {
                Err(AudioError::InitializationFailed(format!(
                    "CoInitializeEx failed: HRESULT 0x{:08X}",
                    hr.0 as u32
                )))
            }
        }
    }

    impl Drop for ComGuard {
        fn drop(&mut self) {
            if self.owns_init {
                // SAFETY: Balances the successful CoInitializeEx call made in
                // ComGuard::new.
                unsafe { CoUninitialize() };
            }
        }
    }

    // -------------------------------------------------------------------------
    // Device enumerator
    // -------------------------------------------------------------------------

    /// Creates a new [`IMMDeviceEnumerator`] instance.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::InitializationFailed`] on COM failure.
    pub(crate) fn create_enumerator() -> Result<IMMDeviceEnumerator, AudioError> {
        // SAFETY: CoCreateInstance is called with a valid, well-known CLSID
        // and context flag.  The returned interface pointer is managed by the
        // windows-crate reference-counting wrapper.
        unsafe {
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| AudioError::InitializationFailed(e.to_string()))
        }
    }

    // -------------------------------------------------------------------------
    // Device identity helpers
    // -------------------------------------------------------------------------

    /// Returns the string endpoint ID for `device`.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::InitializationFailed`] if the ID cannot be
    /// retrieved.
    pub(crate) fn device_id(device: &IMMDevice) -> Result<String, AudioError> {
        // SAFETY: IMMDevice::GetId allocates the PWSTR with CoTaskMemAlloc.
        // We convert the wide string to an owned Rust String and then release
        // the allocation with CoTaskMemFree.  PWSTR::to_string is unsafe because
        // it dereferences a raw pointer; it is sound here because the pointer
        // was just returned by the Windows API and is valid.
        unsafe {
            let pwstr = device
                .GetId()
                .map_err(|e| AudioError::InitializationFailed(e.to_string()))?;

            let id = pwstr
                .to_string()
                .map_err(|e| AudioError::InitializationFailed(e.to_string()))?;

            // Release the CoTaskMem-allocated buffer.
            CoTaskMemFree(Some(pwstr.as_ptr().cast()));

            Ok(id)
        }
    }

    /// Returns the friendly name for `device` by reading
    /// `PKEY_Device_FriendlyName` from its property store.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::InitializationFailed`] if the property store
    /// cannot be opened or the name property cannot be read.
    pub(crate) fn device_name(device: &IMMDevice) -> Result<String, AudioError> {
        // SAFETY:
        // * OpenPropertyStore is called with a valid, documented access mode.
        // * GetValue is called with a well-known property key.
        // * PropVariantToStringAlloc allocates its output with CoTaskMemAlloc;
        //   we release it with CoTaskMemFree before returning.
        // * PWSTR::to_string dereferences a raw pointer that was just returned
        //   by the Windows API; the pointer is valid for the duration of the
        //   call.
        unsafe {
            let store: IPropertyStore = device
                .OpenPropertyStore(STGM_READ)
                .map_err(|e| AudioError::InitializationFailed(e.to_string()))?;

            let key = PKEY_Device_FriendlyName;
            let pv = store
                .GetValue(&raw const key)
                .map_err(|e| AudioError::InitializationFailed(e.to_string()))?;

            let pwstr = PropVariantToStringAlloc(&raw const pv)
                .map_err(|e| AudioError::InitializationFailed(e.to_string()))?;

            let name = pwstr
                .to_string()
                .map_err(|e| AudioError::InitializationFailed(e.to_string()))?;

            CoTaskMemFree(Some(pwstr.as_ptr().cast()));

            Ok(name)
        }
    }

    // -------------------------------------------------------------------------
    // Device lookup
    // -------------------------------------------------------------------------

    /// Returns the default render endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::InitializationFailed`] if the default device
    /// cannot be resolved.
    pub(crate) fn get_default_device(
        enumerator: &IMMDeviceEnumerator,
    ) -> Result<IMMDevice, AudioError> {
        // SAFETY: GetDefaultAudioEndpoint is called with valid, documented
        // enum values for data-flow and role.
        unsafe {
            enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|e| AudioError::InitializationFailed(e.to_string()))
        }
    }

    /// Returns the render endpoint identified by `id`.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::DeviceNotFound`] when no endpoint with the given
    /// ID exists, or [`AudioError::InitializationFailed`] on other COM
    /// failures.
    pub(crate) fn get_device_by_id(
        enumerator: &IMMDeviceEnumerator,
        id: &str,
    ) -> Result<IMMDevice, AudioError> {
        // Encode the ID as a null-terminated UTF-16 sequence.
        let wide_id: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();

        // SAFETY: GetDevice expects a non-null, null-terminated PCWSTR.
        // `wide_id` satisfies both requirements.
        unsafe {
            enumerator
                .GetDevice(windows::core::PCWSTR(wide_id.as_ptr()))
                .map_err(|e| {
                    // Map well-known "device not found" HRESULTs to DeviceNotFound.
                    match e.code().0 {
                        HRESULT_ERROR_NOT_FOUND
                        | HRESULT_ERROR_FILE_NOT_FOUND
                        | AUDCLNT_E_DEVICE_INVALIDATED => AudioError::DeviceNotFound,
                        _ => AudioError::InitializationFailed(e.to_string()),
                    }
                })
        }
    }

    /// Enumerates all active render endpoints and returns an
    /// [`IMMDeviceCollection`].
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::ListFailed`] on COM failure.
    pub(crate) fn enumerate_devices(
        enumerator: &IMMDeviceEnumerator,
    ) -> Result<IMMDeviceCollection, AudioError> {
        // SAFETY: EnumAudioEndpoints is called with valid, documented enum
        // values.
        unsafe {
            enumerator
                .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                .map_err(|e| AudioError::ListFailed(e.to_string()))
        }
    }

    /// Lists all active render endpoints as `(id, name)` pairs.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::ListFailed`] if the collection cannot be obtained,
    /// or [`AudioError::InitializationFailed`] if any device's metadata cannot
    /// be read.
    pub(crate) fn list_devices(
        enumerator: &IMMDeviceEnumerator,
    ) -> Result<Vec<(String, String)>, AudioError> {
        let collection = enumerate_devices(enumerator)?;

        // SAFETY: GetCount is a simple read-only COM call.
        let count = unsafe {
            collection
                .GetCount()
                .map_err(|e| AudioError::ListFailed(e.to_string()))?
        };

        let mut result = Vec::with_capacity(count as usize);

        for i in 0..count {
            // SAFETY: Item is called with a valid index in [0, count).
            let device = unsafe {
                collection
                    .Item(i)
                    .map_err(|e| AudioError::ListFailed(e.to_string()))?
            };

            let id = device_id(&device)?;
            let name = device_name(&device)?;
            result.push((id, name));
        }

        Ok(result)
    }

    // -------------------------------------------------------------------------
    // Volume and mute
    // -------------------------------------------------------------------------

    /// Converts a WASAPI volume scalar (`0.0..=1.0`) to a percentage (`0..=100`).
    fn scalar_to_volume_percent(scalar: f32) -> u8 {
        // Clamp to the valid range before conversion.  The cast is safe
        // because the clamped value is always in [0.0, 100.0], which fits
        // in a u8 without sign loss or truncation beyond rounding.
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let vol = (scalar.clamp(0.0, 1.0) * 100.0_f32).round() as u8;
        vol
    }

    /// Activates and returns the [`IAudioEndpointVolume`] interface for
    /// `device`.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::InitializationFailed`] on COM failure.
    pub(crate) fn endpoint_volume(device: &IMMDevice) -> Result<IAudioEndpointVolume, AudioError> {
        // SAFETY: Activate is called with CLSCTX_INPROC_SERVER and the return
        // type is annotated as IAudioEndpointVolume whose IID is statically
        // correct.  Passing None for pActivationParams is explicitly permitted
        // by the WASAPI documentation.
        unsafe {
            device
                .Activate(CLSCTX_INPROC_SERVER, None)
                .map_err(|e| AudioError::InitializationFailed(e.to_string()))
        }
    }

    /// Returns the master volume level as a value in `0..=100`.
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError::DeviceInvalidated`] when the endpoint's COM
    /// interface has been invalidated (`AUDCLNT_E_DEVICE_INVALIDATED`),
    /// signalling the caller to refresh the cached interface and retry.
    /// Returns [`EndpointError::Error`] wrapping
    /// [`AudioError::GetVolumeFailed`] on any other COM failure.
    pub(crate) fn get_volume(endpoint: &IAudioEndpointVolume) -> Result<u8, EndpointError> {
        // SAFETY: GetMasterVolumeLevelScalar is a simple read-only COM call
        // with no aliasing concerns.
        let scalar = unsafe {
            endpoint.GetMasterVolumeLevelScalar().map_err(|e| {
                if e.code().0 == AUDCLNT_E_DEVICE_INVALIDATED {
                    EndpointError::DeviceInvalidated
                } else {
                    EndpointError::Error(AudioError::GetVolumeFailed(e.to_string()))
                }
            })?
        };

        Ok(scalar_to_volume_percent(scalar))
    }

    /// Sets the master volume level.
    ///
    /// `vol` is clamped to `0..=100` before conversion to the scalar
    /// `0.0..=1.0` range expected by WASAPI.
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError::DeviceInvalidated`] when the endpoint's COM
    /// interface has been invalidated (`AUDCLNT_E_DEVICE_INVALIDATED`),
    /// signalling the caller to refresh the cached interface and retry.
    /// Returns [`EndpointError::Error`] wrapping
    /// [`AudioError::SetVolumeFailed`] on any other COM failure.
    pub(crate) fn set_volume(
        endpoint: &IAudioEndpointVolume,
        vol: u8,
    ) -> Result<(), EndpointError> {
        let scalar = f32::from(vol.min(100)) / 100.0_f32;

        // SAFETY: SetMasterVolumeLevelScalar is a simple setter.  Passing a
        // null pointer for pGUIDEventContext is explicitly permitted by the
        // WASAPI documentation (means "no event context").
        unsafe {
            endpoint
                .SetMasterVolumeLevelScalar(scalar, std::ptr::null())
                .map_err(|e| {
                    if e.code().0 == AUDCLNT_E_DEVICE_INVALIDATED {
                        EndpointError::DeviceInvalidated
                    } else {
                        EndpointError::Error(AudioError::SetVolumeFailed(e.to_string()))
                    }
                })
        }
    }

    /// Returns `true` if the endpoint is currently muted.
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError::DeviceInvalidated`] when the endpoint's COM
    /// interface has been invalidated (`AUDCLNT_E_DEVICE_INVALIDATED`),
    /// signalling the caller to refresh the cached interface and retry.
    /// Returns [`EndpointError::Error`] wrapping [`AudioError::GetMuteFailed`]
    /// on any other COM failure.
    pub(crate) fn get_mute(endpoint: &IAudioEndpointVolume) -> Result<bool, EndpointError> {
        // SAFETY: GetMute is a simple read-only COM call.  The returned BOOL
        // is converted to a Rust bool via as_bool().
        let b = unsafe {
            endpoint.GetMute().map_err(|e| {
                if e.code().0 == AUDCLNT_E_DEVICE_INVALIDATED {
                    EndpointError::DeviceInvalidated
                } else {
                    EndpointError::Error(AudioError::GetMuteFailed(e.to_string()))
                }
            })?
        };

        Ok(b.as_bool())
    }

    /// Mutes or unmutes the endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError::DeviceInvalidated`] when the endpoint's COM
    /// interface has been invalidated (`AUDCLNT_E_DEVICE_INVALIDATED`),
    /// signalling the caller to refresh the cached interface and retry.
    /// Returns [`EndpointError::Error`] wrapping [`AudioError::SetMuteFailed`]
    /// on any other COM failure.
    pub(crate) fn set_mute(
        endpoint: &IAudioEndpointVolume,
        muted: bool,
    ) -> Result<(), EndpointError> {
        // SAFETY: SetMute is a simple setter.  Passing a null pointer for
        // pGUIDEventContext is explicitly permitted by the WASAPI documentation.
        unsafe {
            endpoint.SetMute(muted, std::ptr::null()).map_err(|e| {
                if e.code().0 == AUDCLNT_E_DEVICE_INVALIDATED {
                    EndpointError::DeviceInvalidated
                } else {
                    EndpointError::Error(AudioError::SetMuteFailed(e.to_string()))
                }
            })
        }
    }

    // -------------------------------------------------------------------------
    // Unit tests
    // -------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::scalar_to_volume_percent;

        #[test]
        fn scalar_zero_maps_to_zero_percent() {
            assert_eq!(scalar_to_volume_percent(0.0), 0);
        }

        #[test]
        fn scalar_one_maps_to_hundred_percent() {
            assert_eq!(scalar_to_volume_percent(1.0), 100);
        }

        #[test]
        fn scalar_half_maps_to_fifty_percent() {
            assert_eq!(scalar_to_volume_percent(0.5), 50);
        }

        #[test]
        fn scalar_below_zero_clamps_to_zero() {
            assert_eq!(scalar_to_volume_percent(-1.0), 0);
        }

        #[test]
        fn scalar_above_one_clamps_to_hundred() {
            assert_eq!(scalar_to_volume_percent(2.0), 100);
        }

        #[test]
        fn scalar_rounds_correctly() {
            // 0.754 * 100 = 75.4 → rounds to 75
            assert_eq!(scalar_to_volume_percent(0.754), 75);
            // 0.756 * 100 = 75.6 → rounds to 76
            assert_eq!(scalar_to_volume_percent(0.756), 76);
        }
    }
}
