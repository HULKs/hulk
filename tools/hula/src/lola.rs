use serde::Serialize;

// #[derive(Debug, Default, Deserialize)]
// #[allow(non_snake_case)]
// pub struct LoLAStateMessage<'a> {
//     #[serde(borrow, rename = "RobotConfig")]
//     pub robot_configuration: [&'a str; 4],
//     #[serde(rename = "Accelerometer")]
//     accelerometer: [f32; 3],
//     #[serde(rename = "Angles")]
//     angles: [f32; 2],
//     #[serde(rename = "Battery")]
//     pub battery: [f32; 4],
//     #[serde(rename = "Current")]
//     current: [f32; 25],
//     #[serde(rename = "FSR")]
//     force_sensitive_resistors: [f32; 8],
//     #[serde(rename = "Gyroscope")]
//     gyroscope: [f32; 3],
//     #[serde(rename = "Position")]
//     position: [f32; 25],
//     #[serde(rename = "Sonar")]
//     sonar: [f32; 2],
//     #[serde(rename = "Stiffness")]
//     stiffness: [f32; 25],
//     #[serde(rename = "Temperature")]
//     temperature: [f32; 25],
//     #[serde(rename = "Touch")]
//     touch: [f32; 14],
//     #[serde(rename = "Status")]
//     status: [f32; 25],
// }

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
