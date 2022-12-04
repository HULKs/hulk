use crate::robot_state::JointsArray;

#[derive(Debug, Default)]
#[repr(C)]
pub struct HulaControlFrame {
    left_eye: Eye,
    right_eye: Eye,
    chest: Color,
    left_foot: Color,
    right_foot: Color,
    left_ear: Ear,
    right_ear: Ear,
    position: JointsArray,
    stiffness: JointsArray,
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct Color {
    red: f32,
    green: f32,
    blue: f32,
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct Eye {
    color_at_0: Color,
    color_at_45: Color,
    color_at_90: Color,
    color_at_135: Color,
    color_at_180: Color,
    color_at_225: Color,
    color_at_270: Color,
    color_at_315: Color,
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct Ear {
    intensity_at_0: f32,
    intensity_at_36: f32,
    intensity_at_72: f32,
    intensity_at_108: f32,
    intensity_at_144: f32,
    intensity_at_180: f32,
    intensity_at_216: f32,
    intensity_at_252: f32,
    intensity_at_288: f32,
    intensity_at_324: f32,
}
