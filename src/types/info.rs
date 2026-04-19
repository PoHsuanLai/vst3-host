//! Newtype wrappers over the vst3 crate's `BusInfo` and `ParameterInfo` that
//! preserve the `BusInfo::name_string`, `Vst3ParameterInfo::title_string`, etc.
//! convenience API the rest of the workspace depends on.

use vst3::Steinberg::Vst::ParameterInfo_::ParameterFlags_;

use crate::helpers::utf16_to_string;

/// Host-facing bus descriptor. Thin wrapper around `vst3::Steinberg::Vst::BusInfo`
/// whose field layout is retained verbatim for FFI, but whose only public
/// accessor is [`BusInfo::name_string`].
#[derive(Clone)]
pub struct BusInfo {
    pub(crate) inner: vst3::Steinberg::Vst::BusInfo,
}

impl Default for BusInfo {
    fn default() -> Self {
        Self {
            inner: unsafe { std::mem::zeroed() },
        }
    }
}

impl BusInfo {
    pub fn media_type(&self) -> i32 {
        self.inner.mediaType
    }

    pub fn direction(&self) -> i32 {
        self.inner.direction
    }

    pub fn channel_count(&self) -> i32 {
        self.inner.channelCount
    }

    pub fn flags(&self) -> u32 {
        self.inner.flags
    }

    pub fn bus_type(&self) -> i32 {
        self.inner.busType
    }

    pub fn name_string(&self) -> String {
        utf16_to_string(&self.inner.name)
    }

    pub(crate) fn as_mut_inner(&mut self) -> &mut vst3::Steinberg::Vst::BusInfo {
        &mut self.inner
    }
}

/// Host-facing parameter descriptor — a flat snake_case view over the vst3
/// crate's `ParameterInfo`, populated from the C struct returned by
/// `IEditController::getParameterInfo`.
///
/// Field names intentionally match the original hand-rolled struct so downstream
/// code that reads `info.id`, `info.default_normalized_value`, etc. keeps working.
#[derive(Clone)]
pub struct Vst3ParameterInfo {
    pub id: u32,
    pub title: [u16; 128],
    pub short_title: [u16; 128],
    pub units: [u16; 128],
    /// 0 = continuous.
    pub step_count: i32,
    /// 0.0 - 1.0.
    pub default_normalized_value: f64,
    pub unit_id: i32,
    pub flags: i32,
}

impl Default for Vst3ParameterInfo {
    fn default() -> Self {
        Self {
            id: 0,
            title: [0; 128],
            short_title: [0; 128],
            units: [0; 128],
            step_count: 0,
            default_normalized_value: 0.0,
            unit_id: 0,
            flags: 0,
        }
    }
}

impl Vst3ParameterInfo {
    pub fn title_string(&self) -> String {
        utf16_to_string(&self.title)
    }

    pub fn short_title_string(&self) -> String {
        utf16_to_string(&self.short_title)
    }

    pub fn units_string(&self) -> String {
        utf16_to_string(&self.units)
    }

    pub fn can_automate(&self) -> bool {
        (self.flags & parameter_flags::CAN_AUTOMATE) != 0
    }

    pub fn is_read_only(&self) -> bool {
        (self.flags & parameter_flags::IS_READ_ONLY) != 0
    }

    pub fn is_hidden(&self) -> bool {
        (self.flags & parameter_flags::IS_HIDDEN) != 0
    }

    pub fn is_bypass(&self) -> bool {
        (self.flags & parameter_flags::IS_BYPASS) != 0
    }

    pub fn is_wrap(&self) -> bool {
        (self.flags & parameter_flags::IS_WRAP) != 0
    }

    pub(crate) fn from_c(c: &vst3::Steinberg::Vst::ParameterInfo) -> Self {
        Self {
            id: c.id,
            title: c.title,
            short_title: c.shortTitle,
            units: c.units,
            step_count: c.stepCount,
            default_normalized_value: c.defaultNormalizedValue,
            unit_id: c.unitId,
            flags: c.flags,
        }
    }
}

/// VST3 `ParameterInfo` flag bits as simple `i32` constants.
///
/// Defined here rather than re-exported so the module path `parameter_flags::`
/// exactly mirrors the original crate's public surface.
pub mod parameter_flags {
    use super::ParameterFlags_;

    pub const CAN_AUTOMATE: i32 = ParameterFlags_::kCanAutomate;
    pub const IS_READ_ONLY: i32 = ParameterFlags_::kIsReadOnly;
    pub const IS_WRAP: i32 = ParameterFlags_::kIsWrapAround;
    pub const IS_LIST: i32 = ParameterFlags_::kIsList;
    pub const IS_HIDDEN: i32 = ParameterFlags_::kIsHidden;
    pub const IS_PROGRAM_CHANGE: i32 = ParameterFlags_::kIsProgramChange;
    pub const IS_BYPASS: i32 = ParameterFlags_::kIsBypass;
}
