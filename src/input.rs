use sys::InputHandle_t;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::*;

/// Access to the steam input interface
pub struct Input {
    pub(crate) input: *mut sys::ISteamInput,
    pub(crate) _inner: Arc<Inner>,
}

/// The type of an input device, as returned by [`Input::get_input_type_for_handle`]
/// and in the [`SteamInputGamepadSlotChange`] callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum InputType {
    Unknown,
    SteamController,
    XBox360Controller,
    XBoxOneController,
    GenericGamepad,
    PS4Controller,
    AppleMFiController,
    AndroidController,
    SwitchJoyConPair,
    SwitchJoyConSingle,
    SwitchProController,
    MobileTouch,
    PS3Controller,
    PS5Controller,
    SteamDeckController,
}

impl From<sys::ESteamInputType> for InputType {
    fn from(input_type: sys::ESteamInputType) -> Self {
        match input_type {
            sys::ESteamInputType::k_ESteamInputType_SteamController => Self::SteamController,
            sys::ESteamInputType::k_ESteamInputType_GenericGamepad => Self::GenericGamepad,
            sys::ESteamInputType::k_ESteamInputType_PS4Controller => Self::PS4Controller,
            sys::ESteamInputType::k_ESteamInputType_SwitchJoyConPair => Self::SwitchJoyConPair,
            sys::ESteamInputType::k_ESteamInputType_MobileTouch => Self::MobileTouch,
            sys::ESteamInputType::k_ESteamInputType_PS3Controller => Self::PS3Controller,
            sys::ESteamInputType::k_ESteamInputType_PS5Controller => Self::PS5Controller,
            sys::ESteamInputType::k_ESteamInputType_XBox360Controller => Self::XBox360Controller,
            sys::ESteamInputType::k_ESteamInputType_XBoxOneController => Self::XBoxOneController,
            sys::ESteamInputType::k_ESteamInputType_AppleMFiController => Self::AppleMFiController,
            sys::ESteamInputType::k_ESteamInputType_AndroidController => Self::AndroidController,
            sys::ESteamInputType::k_ESteamInputType_SwitchJoyConSingle => Self::SwitchJoyConSingle,
            sys::ESteamInputType::k_ESteamInputType_SwitchProController => {
                Self::SwitchProController
            }
            sys::ESteamInputType::k_ESteamInputType_SteamDeckController => {
                Self::SteamDeckController
            }
            _ => Self::Unknown,
        }
    }
}

/// Size of the png returned by [`Input::get_glyph_png_for_action_origin`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphSize {
    Small,
    Medium,
    Large,
}

bitflags! {
    /// Style of the glyphs returned by [`Input::get_glyph_png_for_action_origin`]
    /// and [`Input::get_glyph_svg_for_action_origin`]
    ///
    /// The base styles (Knockout, Light, Dark) can not be combined with each other.
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[repr(C)]
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct GlyphStyle: u32 {
        const KNOCKOUT          = 0x0;
        const LIGHT             = 0x1;
        const DARK              = 0x2;
        const NEUTRAL_COLOR_ABXY = 0x10;
        const SOLID_ABXY        = 0x20;
    }
}

// Steam can return windows style separators in glyph paths,
// which are broken as-is on non-windows platforms
#[cfg(not(target_os = "windows"))]
fn normalize_glyph_path(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(target_os = "windows")]
fn normalize_glyph_path(path: &str) -> String {
    path.to_owned()
}

impl Input {
    /// Init must be called when starting use of this interface.
    /// if explicitly_call_run_frame is called then you will need to manually call RunFrame
    /// each frame, otherwise Steam Input will updated when SteamAPI_RunCallbacks() is called
    pub fn init(&self, explicitly_call_run_frame: bool) -> bool {
        unsafe { sys::SteamAPI_ISteamInput_Init(self.input, explicitly_call_run_frame) }
    }

    /// Synchronize API state with the latest Steam Input action data available. This
    /// is performed automatically by SteamAPI_RunCallbacks, but for the absolute lowest
    /// possible latency, you call this directly before reading controller state.
    /// Note: This must be called from somewhere before GetConnectedControllers will
    /// return any handles
    pub fn run_frame(&self) {
        unsafe { sys::SteamAPI_ISteamInput_RunFrame(self.input, false) }
    }

    /// Enables the [`SteamInputDeviceConnected`] and [`SteamInputDeviceDisconnected`] callbacks.
    /// Each controller that is already connected will generate a device connected
    /// callback once enabled.
    pub fn enable_device_callbacks(&self) {
        unsafe { sys::SteamAPI_ISteamInput_EnableDeviceCallbacks(self.input) }
    }

    /// Returns a list of the currently connected controllers
    pub fn get_connected_controllers(&self) -> Vec<sys::InputHandle_t> {
        let mut handles = vec![0_u64; sys::STEAM_INPUT_MAX_COUNT as usize];
        let quantity = self.get_connected_controllers_slice(&mut handles);
        handles.shrink_to(quantity);
        handles
    }

    /// Returns a list of the currently connected controllers without allocating, and the count
    pub fn get_connected_controllers_slice(
        &self,
        mut controllers: impl AsMut<[InputHandle_t]>,
    ) -> usize {
        let handles = controllers.as_mut();
        assert!(handles.len() >= sys::STEAM_INPUT_MAX_COUNT as usize);
        unsafe {
            return sys::SteamAPI_ISteamInput_GetConnectedControllers(
                self.input,
                handles.as_mut_ptr(),
            ) as usize;
        }
    }

    /// Allows to load a specific Action Manifest File localy
    pub fn set_input_action_manifest_file_path(&self, path: &str) -> bool {
        let path = CString::new(path).unwrap();
        unsafe {
            sys::SteamAPI_ISteamInput_SetInputActionManifestFilePath(self.input, path.as_ptr())
        }
    }

    /// Returns the associated ControllerActionSet handle for the specified controller,
    pub fn get_action_set_handle(&self, action_set_name: &str) -> sys::InputActionSetHandle_t {
        let name = CString::new(action_set_name).unwrap();
        unsafe { sys::SteamAPI_ISteamInput_GetActionSetHandle(self.input, name.as_ptr()) }
    }

    /// Returns the input type for a controler
    pub fn get_input_type_for_handle(&self, input_handle: sys::InputHandle_t) -> InputType {
        let input_type: sys::ESteamInputType =
            unsafe { sys::SteamAPI_ISteamInput_GetInputTypeForHandle(self.input, input_handle) };

        input_type.into()
    }

    /// Returns the glyph for an input action
    pub fn get_glyph_for_action_origin(&self, action_origin: sys::EInputActionOrigin) -> String {
        unsafe {
            let glyph_path =
                sys::SteamAPI_ISteamInput_GetGlyphForActionOrigin_Legacy(self.input, action_origin);
            let glyph_path = CStr::from_ptr(glyph_path);
            normalize_glyph_path(&glyph_path.to_string_lossy())
        }
    }

    /// Get a local path to a PNG file for the provided origin's glyph
    pub fn get_glyph_png_for_action_origin(
        &self,
        action_origin: sys::EInputActionOrigin,
        size: GlyphSize,
        glyph_style: GlyphStyle,
    ) -> String {
        unsafe {
            let size = match size {
                GlyphSize::Small => sys::ESteamInputGlyphSize::k_ESteamInputGlyphSize_Small,
                GlyphSize::Medium => sys::ESteamInputGlyphSize::k_ESteamInputGlyphSize_Medium,
                GlyphSize::Large => sys::ESteamInputGlyphSize::k_ESteamInputGlyphSize_Large,
            };
            let glyph_path = sys::SteamAPI_ISteamInput_GetGlyphPNGForActionOrigin(
                self.input,
                action_origin,
                size,
                glyph_style.bits(),
            );
            let glyph_path = CStr::from_ptr(glyph_path);
            normalize_glyph_path(&glyph_path.to_string_lossy())
        }
    }

    /// Get a local path to an SVG file for the provided origin's glyph
    pub fn get_glyph_svg_for_action_origin(
        &self,
        action_origin: sys::EInputActionOrigin,
        glyph_style: GlyphStyle,
    ) -> String {
        unsafe {
            let glyph_path = sys::SteamAPI_ISteamInput_GetGlyphSVGForActionOrigin(
                self.input,
                action_origin,
                glyph_style.bits(),
            );
            let glyph_path = CStr::from_ptr(glyph_path);
            normalize_glyph_path(&glyph_path.to_string_lossy())
        }
    }

    /// Returns the name of an input action
    pub fn get_string_for_action_origin(&self, action_origin: sys::EInputActionOrigin) -> String {
        unsafe {
            let name_path =
                sys::SteamAPI_ISteamInput_GetStringForActionOrigin(self.input, action_origin);
            let name_path = CStr::from_ptr(name_path);
            name_path.to_string_lossy().into_owned()
        }
    }

    /// Reconfigure the controller to use the specified action set
    /// This is cheap, and can be safely called repeatedly.
    pub fn activate_action_set_handle(
        &self,
        input_handle: sys::InputHandle_t,
        action_set_handle: sys::InputActionSetHandle_t,
    ) {
        unsafe {
            sys::SteamAPI_ISteamInput_ActivateActionSet(self.input, input_handle, action_set_handle)
        }
    }

    /// Get the handle of the specified Digital action.
    pub fn get_digital_action_handle(&self, action_name: &str) -> sys::InputDigitalActionHandle_t {
        let name = CString::new(action_name).unwrap();
        unsafe { sys::SteamAPI_ISteamInput_GetDigitalActionHandle(self.input, name.as_ptr()) }
    }

    /// Get the handle of the specified Analog action.
    pub fn get_analog_action_handle(&self, action_name: &str) -> sys::InputAnalogActionHandle_t {
        let name = CString::new(action_name).unwrap();
        unsafe { sys::SteamAPI_ISteamInput_GetAnalogActionHandle(self.input, name.as_ptr()) }
    }

    /// Returns the current state of the supplied digital game action.
    pub fn get_digital_action_data(
        &self,
        input_handle: sys::InputHandle_t,
        action_handle: sys::InputDigitalActionHandle_t,
    ) -> sys::InputDigitalActionData_t {
        unsafe {
            sys::SteamAPI_ISteamInput_GetDigitalActionData(self.input, input_handle, action_handle)
        }
    }

    /// Returns the current state of the supplied analog game action.
    pub fn get_analog_action_data(
        &self,
        input_handle: sys::InputHandle_t,
        action_handle: sys::InputAnalogActionHandle_t,
    ) -> sys::InputAnalogActionData_t {
        unsafe {
            sys::SteamAPI_ISteamInput_GetAnalogActionData(self.input, input_handle, action_handle)
        }
    }

    /// Get the origin(s) for a digital action within an action set.
    pub fn get_digital_action_origins(
        &self,
        input_handle: sys::InputHandle_t,
        action_set_handle: sys::InputActionSetHandle_t,
        digital_action_handle: sys::InputDigitalActionHandle_t,
    ) -> Vec<sys::EInputActionOrigin> {
        unsafe {
            let mut origins = Vec::with_capacity(sys::STEAM_INPUT_MAX_ORIGINS as usize);
            let len = sys::SteamAPI_ISteamInput_GetDigitalActionOrigins(
                self.input,
                input_handle,
                action_set_handle,
                digital_action_handle,
                origins.as_mut_ptr(),
            );
            origins.set_len(len as usize);
            origins
        }
    }

    /// Get the origin(s) for an analog action within an action set.
    pub fn get_analog_action_origins(
        &self,
        input_handle: sys::InputHandle_t,
        action_set_handle: sys::InputActionSetHandle_t,
        analog_action_handle: sys::InputAnalogActionHandle_t,
    ) -> Vec<sys::EInputActionOrigin> {
        unsafe {
            let mut origins = Vec::with_capacity(sys::STEAM_INPUT_MAX_ORIGINS as usize);
            let len = sys::SteamAPI_ISteamInput_GetAnalogActionOrigins(
                self.input,
                input_handle,
                action_set_handle,
                analog_action_handle,
                origins.as_mut_ptr(),
            );
            origins.set_len(len as usize);
            origins
        }
    }

    pub fn get_motion_data(&self, input_handle: sys::InputHandle_t) -> sys::InputMotionData_t {
        unsafe { sys::SteamAPI_ISteamInput_GetMotionData(self.input, input_handle) }
    }

    /// Invokes the Steam overlay and brings up the binding screen.
    /// Returns true for success, false if overlay is disabled/unavailable.
    /// If the player is using Big Picture Mode the configuration will open in
    /// the overlay. In desktop mode a popup window version of Big Picture will
    /// be created and open the configuration.
    pub fn show_binding_panel(&self, input_handle: sys::InputHandle_t) -> bool {
        unsafe { sys::SteamAPI_ISteamInput_ShowBindingPanel(self.input, input_handle) }
    }

    /// Shutdown must be called when ending use of this interface.
    pub fn shutdown(&self) {
        unsafe {
            sys::SteamAPI_ISteamInput_Shutdown(self.input);
        }
    }
}

/// Called when a controller configuration is loaded for a device.
///
/// Note: these callbacks will always fire if you have a handler,
/// but only while the Steam Input interface is active.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SteamInputConfigurationLoaded {
    pub app_id: AppId,
    pub device_handle: InputHandle_t,
    /// May differ from the local user when using an unmodified community or official config.
    pub mapping_creator: SteamId,
    pub major_revision: u32,
    pub minor_revision: u32,
    pub uses_steam_input_api: bool,
    pub uses_gamepad_api: bool,
}

impl_callback!(cb: SteamInputConfigurationLoaded_t => SteamInputConfigurationLoaded {
    Self {
        app_id: AppId(cb.m_unAppID),
        device_handle: cb.m_ulDeviceHandle,
        mapping_creator: SteamId(cb.m_ulMappingCreator.m_steamid.m_unAll64Bits),
        major_revision: cb.m_unMajorRevision,
        minor_revision: cb.m_unMinorRevision,
        uses_steam_input_api: cb.m_bUsesSteamInputAPI,
        uses_gamepad_api: cb.m_bUsesGamepadAPI,
    }
});

/// Called when a controller is connected.
///
/// Requires [`Input::enable_device_callbacks`] to be called first. Each
/// controller that is already connected will generate this callback once
/// enabled.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SteamInputDeviceConnected {
    pub device_handle: InputHandle_t,
}

impl_callback!(cb: SteamInputDeviceConnected_t => SteamInputDeviceConnected {
    Self {
        device_handle: cb.m_ulConnectedDeviceHandle,
    }
});

/// Called when a controller is disconnected.
///
/// Requires [`Input::enable_device_callbacks`] to be called first.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SteamInputDeviceDisconnected {
    pub device_handle: InputHandle_t,
}

impl_callback!(cb: SteamInputDeviceDisconnected_t => SteamInputDeviceDisconnected {
    Self {
        device_handle: cb.m_ulDisconnectedDeviceHandle,
    }
});

/// Called when the gamepad slot of a controller changes, e.g. on first input
/// for a device using gamepad emulation, or when the user changes the order
/// via the Steam overlay. Can be used to change out glyph sets without
/// making additional API calls.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SteamInputGamepadSlotChange {
    pub app_id: AppId,
    pub device_handle: InputHandle_t,
    pub device_type: InputType,
    /// Previous gamepad slot, or -1 if the controller doesn't use gamepad bindings.
    pub old_gamepad_slot: i32,
    /// New gamepad slot, or -1 if the controller doesn't use gamepad bindings.
    pub new_gamepad_slot: i32,
}

impl_callback!(cb: SteamInputGamepadSlotChange_t => SteamInputGamepadSlotChange {
    Self {
        app_id: AppId(cb.m_unAppID),
        device_handle: cb.m_ulDeviceHandle,
        device_type: cb.m_eDeviceType.into(),
        old_gamepad_slot: cb.m_nOldGamepadSlot,
        new_gamepad_slot: cb.m_nNewGamepadSlot,
    }
});

#[cfg(all(test, not(target_os = "windows")))]
mod tests {
    use super::normalize_glyph_path;

    #[test]
    fn glyph_path_backslashes_are_replaced() {
        assert_eq!(
            normalize_glyph_path(r"\controller_base\images\api\light/a.png"),
            "/controller_base/images/api/light/a.png"
        );
    }
}
