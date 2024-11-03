use crate::{lola::LolaControlFrame, robot_state::JointsArray};

#[derive(Debug, Default)]
#[repr(C)]
pub struct HulaControlFrame {
    pub left_eye: Eye,
    pub right_eye: Eye,
    pub chest: Color,
    pub left_foot: Color,
    pub right_foot: Color,
    pub left_ear: Ear,
    pub right_ear: Ear,
    pub position: JointsArray,
    pub stiffness: JointsArray,
}

impl HulaControlFrame {
    pub fn into_lola(self, skull: [f32; 12]) -> LolaControlFrame {
        LolaControlFrame {
            chest: self.chest.into_lola(),
            left_ear: self.left_ear.into_left_ear(),
            left_eye: self.left_eye.into_left_eye(),
            left_foot: self.left_foot.into_lola(),
            position: self.position.into_lola(),
            right_ear: self.right_ear.into_right_ear(),
            right_eye: self.right_eye.into_right_eye(),
            right_foot: self.right_foot.into_lola(),
            skull,
            sonar: [true; 2],
            stiffness: self.stiffness.into_lola(),
        }
    }
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct Color {
    red: f32,
    green: f32,
    blue: f32,
}

impl Color {
    fn into_lola(self) -> [f32; 3] {
        [self.red, self.green, self.blue]
    }
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

impl Eye {
    fn into_left_eye(self) -> [f32; 24] {
        [
            self.color_at_45.red,
            self.color_at_0.red,
            self.color_at_315.red,
            self.color_at_270.red,
            self.color_at_225.red,
            self.color_at_180.red,
            self.color_at_135.red,
            self.color_at_90.red,
            //
            self.color_at_45.green,
            self.color_at_0.green,
            self.color_at_315.green,
            self.color_at_270.green,
            self.color_at_225.green,
            self.color_at_180.green,
            self.color_at_135.green,
            self.color_at_90.green,
            //
            self.color_at_45.blue,
            self.color_at_0.blue,
            self.color_at_315.blue,
            self.color_at_270.blue,
            self.color_at_225.blue,
            self.color_at_180.blue,
            self.color_at_135.blue,
            self.color_at_90.blue,
        ]
    }

    fn into_right_eye(self) -> [f32; 24] {
        [
            self.color_at_0.red,
            self.color_at_45.red,
            self.color_at_90.red,
            self.color_at_135.red,
            self.color_at_180.red,
            self.color_at_225.red,
            self.color_at_270.red,
            self.color_at_315.red,
            //
            self.color_at_0.green,
            self.color_at_45.green,
            self.color_at_90.green,
            self.color_at_135.green,
            self.color_at_180.green,
            self.color_at_225.green,
            self.color_at_270.green,
            self.color_at_315.green,
            //
            self.color_at_0.blue,
            self.color_at_45.blue,
            self.color_at_90.blue,
            self.color_at_135.blue,
            self.color_at_180.blue,
            self.color_at_225.blue,
            self.color_at_270.blue,
            self.color_at_315.blue,
        ]
    }
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

impl Ear {
    fn into_left_ear(self) -> [f32; 10] {
        [
            self.intensity_at_0,
            self.intensity_at_36,
            self.intensity_at_72,
            self.intensity_at_108,
            self.intensity_at_144,
            self.intensity_at_180,
            self.intensity_at_216,
            self.intensity_at_252,
            self.intensity_at_288,
            self.intensity_at_324,
        ]
    }
    fn into_right_ear(self) -> [f32; 10] {
        [
            self.intensity_at_324,
            self.intensity_at_288,
            self.intensity_at_252,
            self.intensity_at_216,
            self.intensity_at_180,
            self.intensity_at_144,
            self.intensity_at_108,
            self.intensity_at_72,
            self.intensity_at_36,
            self.intensity_at_0,
        ]
    }
}
