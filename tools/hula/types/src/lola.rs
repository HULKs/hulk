use serde::Serialize;

#[derive(Debug, Serialize)]
#[allow(non_snake_case)]
pub struct LolaControlFrame {
    #[serde(rename = "Chest")]
    pub chest: [f32; 3],
    #[serde(rename = "LEar")]
    pub left_ear: [f32; 10],
    #[serde(rename = "LEye")]
    pub left_eye: [f32; 24],
    #[serde(rename = "LFoot")]
    pub left_foot: [f32; 3],
    #[serde(rename = "Position")]
    pub position: [f32; 25],
    #[serde(rename = "REar")]
    pub right_ear: [f32; 10],
    #[serde(rename = "REye")]
    pub right_eye: [f32; 24],
    #[serde(rename = "RFoot")]
    pub right_foot: [f32; 3],
    #[serde(rename = "Skull")]
    pub skull: [f32; 12],
    #[serde(rename = "Sonar")]
    pub sonar: [bool; 2],
    #[serde(rename = "Stiffness")]
    pub stiffness: [f32; 25],
}

impl Default for LolaControlFrame {
    fn default() -> Self {
        Self {
            chest: Default::default(),
            left_ear: Default::default(),
            left_eye: Default::default(),
            left_foot: Default::default(),
            position: Default::default(),
            right_ear: Default::default(),
            right_eye: Default::default(),
            right_foot: Default::default(),
            skull: Default::default(),
            sonar: [true; 2],
            stiffness: Default::default(),
        }
    }
}
