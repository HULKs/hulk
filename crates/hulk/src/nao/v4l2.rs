use anyhow::Context;
use log::debug;
use v4l::{
    v4l_sys::{
        V4L2_CID_AUTO_WHITE_BALANCE, V4L2_CID_BRIGHTNESS, V4L2_CID_CONTRAST,
        V4L2_CID_EXPOSURE_ABSOLUTE, V4L2_CID_EXPOSURE_AUTO, V4L2_CID_FOCUS_ABSOLUTE,
        V4L2_CID_FOCUS_AUTO, V4L2_CID_GAIN, V4L2_CID_HUE, V4L2_CID_HUE_AUTO, V4L2_CID_SATURATION,
        V4L2_CID_SHARPNESS, V4L2_CID_WHITE_BALANCE_TEMPERATURE,
    },
    Control, Device,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExposureMode {
    Auto = 0,
    Manual = 1,
}

#[derive(Clone, Copy, Debug)]
pub enum WhiteBalanceMode {
    Auto = 1,
    #[allow(dead_code)]
    Manual = 0,
}

#[derive(Clone, Copy, Debug)]
pub enum HueMode {
    Auto = 0,
    #[allow(dead_code)]
    Manual = 1,
}

#[derive(Clone, Copy, Debug)]
pub enum FocusMode {
    #[allow(dead_code)]
    Auto = 1,
    Manual = 0,
}

#[derive(Debug)]
pub struct V4L2Controls {
    pub exposure_mode: ExposureMode,
    pub white_balance_mode: WhiteBalanceMode,
    pub brightness: i32,
    pub contrast: i32,
    pub gain: i32,
    pub hue: i32,
    pub saturation: i32,
    pub sharpness: i32,
    pub white_balance_temperature: i32,
    pub exposure_absolute: i32,
    pub hue_mode: HueMode,
    pub focus_mode: FocusMode,
    pub focus_absolute: i32,
}

macro_rules! set_v4l2_config {
    ($device:expr, $id:expr, $value:expr) => {
        $device
            .set_control($id, Control::Value($value as i32))
            .with_context(|| format!("Failed to set '{}' to '{}'", stringify!($id), $value as i32))
    };
}

pub fn apply_v4l2_settings(device: &Device, controls: V4L2Controls) -> anyhow::Result<()> {
    debug!("Applying V4L2 controls: {:?}", controls);
    set_v4l2_config!(device, V4L2_CID_EXPOSURE_AUTO, controls.exposure_mode)?;
    set_v4l2_config!(
        device,
        V4L2_CID_AUTO_WHITE_BALANCE,
        controls.white_balance_mode
    )?;
    set_v4l2_config!(device, V4L2_CID_BRIGHTNESS, controls.brightness)?;
    set_v4l2_config!(device, V4L2_CID_CONTRAST, controls.contrast)?;
    set_v4l2_config!(device, V4L2_CID_GAIN, controls.gain)?;
    set_v4l2_config!(device, V4L2_CID_HUE, controls.hue)?;
    set_v4l2_config!(device, V4L2_CID_SATURATION, controls.saturation)?;
    set_v4l2_config!(device, V4L2_CID_SHARPNESS, controls.sharpness)?;
    set_v4l2_config!(
        device,
        V4L2_CID_WHITE_BALANCE_TEMPERATURE,
        controls.white_balance_temperature
    )?;
    if controls.exposure_mode == ExposureMode::Manual {
        set_v4l2_config!(
            device,
            V4L2_CID_EXPOSURE_ABSOLUTE,
            controls.exposure_absolute
        )?;
    }
    set_v4l2_config!(device, V4L2_CID_HUE_AUTO, controls.hue_mode)?;
    set_v4l2_config!(device, V4L2_CID_FOCUS_AUTO, controls.focus_mode)?;
    set_v4l2_config!(device, V4L2_CID_FOCUS_ABSOLUTE, controls.focus_absolute)?;
    Ok(())
}
