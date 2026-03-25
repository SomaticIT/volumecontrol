//! Internal WASAPI helpers for `volumecontrol-windows`.
//!
//! All `unsafe` code is confined to this module.  Every `unsafe` block carries
//! a `// SAFETY:` comment explaining why the operation is sound.
//!
//! The public-facing `AudioDevice` implementation in `lib.rs` calls only the
//! safe wrappers defined here.

#[cfg(feature = "wasapi")]
pub(crate) mod wasapi {
    use volumecontrol_core::AudioError;

    use windows::Win32::{
        Devices::Properties::PKEY_Device_FriendlyName,
        Foundation::BOOL,
        Media::Audio::{
            eConsole, eRender, IAudioEndpointVolume, IMMDevice, IMMDeviceCollection,
            IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_MULTITHREADED, STGM_READ,
        },
        UI::Shell::PropertiesSystem::{IPropertyStore, PropVariantToStringAlloc},
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
            // correct.
            let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };

            match result {
                // S_OK or S_FALSE — this thread now owns the COM initialisation.
                Ok(()) => Ok(Self { owns_init: true }),
                // RPC_E_CHANGED_MODE: COM was already initialised
                // with a different apartment model.  We can still use COM;
                // we must NOT call CoUninitialize on drop.
                Err(ref e) if e.code().0 == RPC_E_CHANGED_MODE => Ok(Self { owns_init: false }),
                Err(e) => Err(AudioError::InitializationFailed(e.to_string())),
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
    /// The caller is responsible for the device reference lifetime.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::InitializationFailed`] if the ID cannot be
    /// retrieved.
    pub(crate) fn device_id(device: &IMMDevice) -> Result<String, AudioError> {
        // SAFETY: IMMDevice::GetId allocates the PWSTR with CoTaskMemAlloc.
        // We convert the wide string to an owned Rust String and then release
        // the allocation with CoTaskMemFree.
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
        unsafe {
            let store: IPropertyStore = device
                .OpenPropertyStore(STGM_READ)
                .map_err(|e| AudioError::InitializationFailed(e.to_string()))?;

            let pv = store
                .GetValue(&PKEY_Device_FriendlyName)
                .map_err(|e| AudioError::InitializationFailed(e.to_string()))?;

            let pwstr = PropVariantToStringAlloc(&pv)
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
        // SAFETY: Activate is called with CLSCTX_INPROC_SERVER and a type
        // parameter whose IID is statically known to be correct.
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
    /// Returns [`AudioError::GetVolumeFailed`] on COM failure.
    pub(crate) fn get_volume(endpoint: &IAudioEndpointVolume) -> Result<u8, AudioError> {
        // SAFETY: GetMasterVolumeLevelScalar is a simple read-only COM call
        // with no aliasing concerns.
        let scalar = unsafe {
            endpoint
                .GetMasterVolumeLevelScalar()
                .map_err(|e| AudioError::GetVolumeFailed(e.to_string()))?
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
    /// Returns [`AudioError::SetVolumeFailed`] on COM failure.
    pub(crate) fn set_volume(endpoint: &IAudioEndpointVolume, vol: u8) -> Result<(), AudioError> {
        let scalar = f32::from(vol.min(100)) / 100.0_f32;

        // SAFETY: SetMasterVolumeLevelScalar is a simple setter.  Passing
        // `None` for the event-context GUID is explicitly permitted by the
        // WASAPI documentation.
        unsafe {
            endpoint
                .SetMasterVolumeLevelScalar(scalar, None)
                .map_err(|e| AudioError::SetVolumeFailed(e.to_string()))
        }
    }

    /// Returns `true` if the endpoint is currently muted.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::GetMuteFailed`] on COM failure.
    pub(crate) fn get_mute(endpoint: &IAudioEndpointVolume) -> Result<bool, AudioError> {
        // SAFETY: GetMute is a simple read-only COM call.
        let b: BOOL = unsafe {
            endpoint
                .GetMute()
                .map_err(|e| AudioError::GetMuteFailed(e.to_string()))?
        };

        Ok(b.as_bool())
    }

    /// Mutes or unmutes the endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::SetMuteFailed`] on COM failure.
    pub(crate) fn set_mute(endpoint: &IAudioEndpointVolume, muted: bool) -> Result<(), AudioError> {
        // SAFETY: SetMute is a simple setter.  Passing `None` for the
        // event-context GUID is explicitly permitted by the WASAPI
        // documentation.
        unsafe {
            endpoint
                .SetMute(BOOL::from(muted), None)
                .map_err(|e| AudioError::SetMuteFailed(e.to_string()))
        }
    }
}
