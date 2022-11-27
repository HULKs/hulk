use serde::Deserialize;

use crate::bindings::{
    v4l2_exposure_auto_type_V4L2_EXPOSURE_APERTURE_PRIORITY,
    v4l2_exposure_auto_type_V4L2_EXPOSURE_AUTO, v4l2_exposure_auto_type_V4L2_EXPOSURE_MANUAL,
    v4l2_exposure_auto_type_V4L2_EXPOSURE_SHUTTER_PRIORITY,
};

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub width: u32,
    pub height: u32,
    pub format: Format,
    pub time_per_frame: Fraction,

    pub brightness: i32,
    pub contrast: i32,
    pub saturation: i32,
    pub hue: i32,
    pub white_balance_temperature_auto: bool,
    pub gain: i32,
    pub hue_auto: bool,
    pub white_balance_temperature: i32,
    pub sharpness: i32,
    pub exposure_auto: ExposureMode,
    pub exposure_absolute: i32,
    pub focus_absolute: i32,
    pub focus_auto: bool,

    pub automatic_exposure_control_weights: [u8; 16],
    pub disable_digital_effects: bool,
    pub flip_sensor: bool,

    pub amount_of_buffers: u32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub enum Format {
    YUVU,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Fraction {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub enum ExposureMode {
    Auto = v4l2_exposure_auto_type_V4L2_EXPOSURE_AUTO as isize,
    Manual = v4l2_exposure_auto_type_V4L2_EXPOSURE_MANUAL as isize,
    ShutterPriority = v4l2_exposure_auto_type_V4L2_EXPOSURE_SHUTTER_PRIORITY as isize,
    AperturePriority = v4l2_exposure_auto_type_V4L2_EXPOSURE_APERTURE_PRIORITY as isize,
}
