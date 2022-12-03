use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Default, Deserialize)]
#[repr(C)]
pub struct RobotState {
    #[serde(skip)]
    pub received_at: f32, // TODO convert to Duration?
    #[serde(rename = "RobotConfig")]
    pub robot_configuration: RobotConfiguration,
    #[serde(rename = "Battery")]
    pub battery: Battery,
    #[serde(flatten)]
    pub inertial_measurement_unit: InertialMeasurementUnit,
    #[serde(rename = "FSR")]
    pub force_sensitive_resistors: ForceSensitiveResistors,
    #[serde(rename = "Touch")]
    pub touch_sensors: TouchSensors,
    #[serde(rename = "Sonar")]
    pub sonar_sensors: SonarSensors,
    #[serde(rename = "Position")]
    pub position: JointsArray,
    #[serde(rename = "Stiffness")]
    pub stiffness: JointsArray,
    #[serde(rename = "Current")]
    pub current: JointsArray,
    #[serde(rename = "Temperature")]
    pub temperature: JointsArray,
    #[serde(rename = "Status")]
    pub status: JointsArray,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[repr(C)]
pub struct RobotConfiguration {
    #[serde(deserialize_with = "deserialize_id")]
    pub body_id: [u8; 20],
    #[serde(deserialize_with = "deserialize_version")]
    pub body_version: u8,
    #[serde(deserialize_with = "deserialize_id")]
    pub head_id: [u8; 20],
    #[serde(deserialize_with = "deserialize_version")]
    pub head_version: u8,
}

fn deserialize_id<'de, D>(deserializer: D) -> Result<[u8; 20], D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .as_bytes()
        .try_into()
        .map_err(serde::de::Error::custom)
}

fn deserialize_version<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let version = String::deserialize(deserializer)?;
    version
        .chars()
        .next()
        .ok_or_else(|| serde::de::Error::custom("expected non-empty version"))?
        .to_digit(10)
        .map(|number| number as u8)
        .ok_or_else(|| serde::de::Error::custom("version is not a number"))
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct Battery {
    pub charge: f32,
    pub status: f32,
    pub current: f32,
    pub temperature: f32,
}

#[derive(Debug, Default, Deserialize)]
#[repr(C)]
pub struct InertialMeasurementUnit {
    #[serde(rename = "Accelerometer")]
    accelerometer: Vertex3,
    #[serde(rename = "Angles")]
    angles: Vertex2,
    #[serde(rename = "Gyroscope")]
    gyroscope: Vertex3,
}

#[derive(Debug, Default, Deserialize)]
#[repr(C)]
pub struct ForceSensitiveResistors {
    left_foot_front_left: f32,
    left_foot_front_right: f32,
    left_foot_rear_left: f32,
    left_foot_rear_right: f32,
    right_foot_front_left: f32,
    right_foot_front_right: f32,
    right_foot_rear_left: f32,
    right_foot_rear_right: f32,
}

#[derive(Debug, Default, Deserialize)]
#[repr(C)]
pub struct TouchSensors {
    #[serde(deserialize_with = "float_as_bool")]
    chest_button: bool,
    #[serde(deserialize_with = "float_as_bool")]
    head_front: bool,
    #[serde(deserialize_with = "float_as_bool")]
    head_middle: bool,
    #[serde(deserialize_with = "float_as_bool")]
    head_rear: bool,
    #[serde(deserialize_with = "float_as_bool")]
    left_foot_left: bool,
    #[serde(deserialize_with = "float_as_bool")]
    left_foot_right: bool,
    #[serde(deserialize_with = "float_as_bool")]
    left_hand_back: bool,
    #[serde(deserialize_with = "float_as_bool")]
    left_hand_left: bool,
    #[serde(deserialize_with = "float_as_bool")]
    left_hand_right: bool,
    #[serde(deserialize_with = "float_as_bool")]
    right_foot_left: bool,
    #[serde(deserialize_with = "float_as_bool")]
    right_foot_right: bool,
    #[serde(deserialize_with = "float_as_bool")]
    right_hand_back: bool,
    #[serde(deserialize_with = "float_as_bool")]
    right_hand_left: bool,
    #[serde(deserialize_with = "float_as_bool")]
    right_hand_right: bool,
}

fn float_as_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(f32::deserialize(deserializer)? >= 0.5)
}

#[derive(Debug, Default, Deserialize)]
#[repr(C)]
pub struct SonarSensors {
    left: f32,
    right: f32,
}

#[derive(Debug, Default, Deserialize)]
#[repr(C)]
pub struct JointsArray {
    head_yaw: f32,
    head_pitch: f32,
    left_shoulder_pitch: f32,
    left_shoulder_roll: f32,
    left_elbow_yaw: f32,
    left_elbow_roll: f32,
    left_wrist_yaw: f32,
    left_hip_yaw_pitch: f32,
    left_hip_roll: f32,
    left_hip_pitch: f32,
    left_knee_pitch: f32,
    left_ankle_pitch: f32,
    left_ankle_roll: f32,
    right_hip_roll: f32,
    right_hip_pitch: f32,
    right_knee_pitch: f32,
    right_ankle_pitch: f32,
    right_ankle_roll: f32,
    right_shoulder_pitch: f32,
    right_shoulder_roll: f32,
    right_elbow_yaw: f32,
    right_elbow_roll: f32,
    right_wrist_yaw: f32,
    left_hand: f32,
    right_hand: f32,
}

impl JointsArray {
    pub fn into_lola(self) -> [f32; 25] {
        [
            self.head_yaw,
            self.head_pitch,
            self.left_shoulder_pitch,
            self.left_shoulder_roll,
            self.left_elbow_yaw,
            self.left_elbow_roll,
            self.left_wrist_yaw,
            self.left_hip_yaw_pitch,
            self.left_hip_roll,
            self.left_hip_pitch,
            self.left_knee_pitch,
            self.left_ankle_pitch,
            self.left_ankle_roll,
            self.right_hip_roll,
            self.right_hip_pitch,
            self.right_knee_pitch,
            self.right_ankle_pitch,
            self.right_ankle_roll,
            self.right_shoulder_pitch,
            self.right_shoulder_roll,
            self.right_elbow_yaw,
            self.right_elbow_roll,
            self.right_wrist_yaw,
            self.left_hand,
            self.right_hand,
        ]
    }
}

#[derive(Debug, Default, Deserialize)]
#[repr(C)]
pub struct Vertex2 {
    x: f32,
    y: f32,
}

#[derive(Debug, Default, Deserialize)]
#[repr(C)]
pub struct Vertex3 {
    x: f32,
    y: f32,
    z: f32,
}
