use std::{convert::TryInto, f32::consts::PI, time::Duration};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
#[allow(non_snake_case)]
pub struct LoLAStateMessage<'a> {
    #[serde(borrow, rename = "RobotConfig")]
    pub robot_configuration: [&'a str; 4],
    #[serde(rename = "Accelerometer")]
    accelerometer: [f32; 3],
    #[serde(rename = "Angles")]
    angles: [f32; 2],
    #[serde(rename = "Battery")]
    pub battery: [f32; 4],
    #[serde(rename = "Current")]
    current: [f32; 25],
    #[serde(rename = "FSR")]
    force_sensitive_resistors: [f32; 8],
    #[serde(rename = "Gyroscope")]
    gyroscope: [f32; 3],
    #[serde(rename = "Position")]
    position: [f32; 25],
    #[serde(rename = "Sonar")]
    sonar: [f32; 2],
    #[serde(rename = "Stiffness")]
    stiffness: [f32; 25],
    #[serde(rename = "Temperature")]
    temperature: [f32; 25],
    #[serde(rename = "Touch")]
    touch: [f32; 14],
    #[serde(rename = "Status")]
    status: [f32; 25],
}

#[derive(Debug, Serialize)]
#[allow(non_snake_case)]
pub struct LoLAControlMessage {
    #[serde(rename = "Chest")]
    chest: [f32; 3],
    #[serde(rename = "LEar")]
    left_ear: [f32; 10],
    #[serde(rename = "LEye")]
    left_eye: [f32; 24],
    #[serde(rename = "LFoot")]
    left_foot: [f32; 3],
    #[serde(rename = "Position")]
    position: [f32; 25],
    #[serde(rename = "REar")]
    right_ear: [f32; 10],
    #[serde(rename = "REye")]
    right_eye: [f32; 24],
    #[serde(rename = "RFoot")]
    right_foot: [f32; 3],
    #[serde(rename = "Skull")]
    skull: [f32; 12],
    #[serde(rename = "Sonar")]
    sonar: [bool; 2],
    #[serde(rename = "Stiffness")]
    stiffness: [f32; 25],
}

impl Default for LoLAControlMessage {
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

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct RobotConfiguration {
    pub body_id: [u8; 20],
    pub body_version: u8,
    pub head_id: [u8; 20],
    pub head_version: u8,
}

impl From<[&str; 4]> for RobotConfiguration {
    fn from(source: [&str; 4]) -> Self {
        let body_id = source[0].as_bytes();

        assert_eq!(source[1].len(), 5, "source[1].len() != 5");
        let major_version = source[1].chars().nth(0).unwrap().to_digit(10).unwrap();
        assert_eq!(major_version, 6, "body_version: major_version != 6");
        let body_version = major_version as u8;

        let head_id = source[2].as_bytes();

        assert_eq!(source[3].len(), 5, "source[3].len() != 5");
        let major_version = source[3].chars().nth(0).unwrap().to_digit(10).unwrap();
        assert_eq!(major_version, 6, "head_version: major_version != 6");
        let head_version = major_version as u8;

        Self {
            body_id: body_id
                .try_into()
                .expect("Unexpected length of body_id string"),
            body_version,
            head_id: head_id
                .try_into()
                .expect("Unexpected length of head_id string"),
            head_version,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Battery {
    pub charge: f32,
    pub status: f32,
    pub current: f32,
    pub temperature: f32,
}

impl From<[f32; 4]> for Battery {
    fn from(source: [f32; 4]) -> Self {
        Self {
            charge: source[0],
            status: source[1],
            current: source[2],
            temperature: source[3],
        }
    }
}

impl Battery {
    pub fn fill_into_skull(&self, seconds: &f64, control_message: &mut LoLAControlMessage) {
        //   front
        //    0 11
        //  1     10
        // 2       9
        // 3       8
        //  4     7
        //    5 6
        //   back
        // 6 is beginning, clock-wise
        let led_positions = [
            0.433628318584071,
            0.349557522123894,
            0.274336283185841,
            0.168141592920354,
            0.0884955752212389,
            0.0442477876106195,
            0.955752212389381,
            0.911504424778761,
            0.831858407079646,
            0.725663716814159,
            0.650442477876106,
            0.566371681415929,
        ];
        for led in 0..12 {
            control_message.skull[led] = if self.charge > led_positions[led] {
                if self.current > 0.0 {
                    let offsetted_seconds = seconds - (led_positions[led] as f64);
                    let fraction = 1.0 - (offsetted_seconds - offsetted_seconds.floor());
                    ((fraction * 0.8) + 0.2) as f32
                } else {
                    1.0
                }
            } else {
                0.0
            };
        }
    }
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct Vertex2 {
    x: f32,
    y: f32,
}

impl From<[f32; 2]> for Vertex2 {
    fn from(source: [f32; 2]) -> Self {
        Self {
            x: source[0],
            y: source[1],
        }
    }
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct Vertex3 {
    x: f32,
    y: f32,
    z: f32,
}

impl From<[f32; 3]> for Vertex3 {
    fn from(source: [f32; 3]) -> Self {
        Self {
            x: source[0],
            y: source[1],
            z: source[2],
        }
    }
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct InertialMeasurementUnit {
    accelerometer: Vertex3,
    angles: Vertex2,
    gyroscope: Vertex3,
}

impl From<&LoLAStateMessage<'_>> for InertialMeasurementUnit {
    fn from(state_message: &LoLAStateMessage) -> Self {
        Self {
            accelerometer: state_message.accelerometer.into(),
            angles: state_message.angles.into(),
            gyroscope: state_message.gyroscope.into(),
        }
    }
}

#[derive(Debug, Default)]
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

impl From<[f32; 8]> for ForceSensitiveResistors {
    fn from(source: [f32; 8]) -> Self {
        Self {
            left_foot_front_left: source[0],
            left_foot_front_right: source[1],
            left_foot_rear_left: source[2],
            left_foot_rear_right: source[3],
            right_foot_front_left: source[4],
            right_foot_front_right: source[5],
            right_foot_rear_left: source[6],
            right_foot_rear_right: source[7],
        }
    }
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct TouchSensors {
    chest_button: bool,
    head_front: bool,
    head_middle: bool,
    head_rear: bool,
    left_foot_left: bool,
    left_foot_right: bool,
    left_hand_back: bool,
    left_hand_left: bool,
    left_hand_right: bool,
    right_foot_left: bool,
    right_foot_right: bool,
    right_hand_back: bool,
    right_hand_left: bool,
    right_hand_right: bool,
}

impl From<[f32; 14]> for TouchSensors {
    fn from(source: [f32; 14]) -> Self {
        Self {
            chest_button: source[0] >= 0.5,
            head_front: source[1] >= 0.5,
            head_middle: source[2] >= 0.5,
            head_rear: source[3] >= 0.5,
            left_foot_left: source[4] >= 0.5,
            left_foot_right: source[5] >= 0.5,
            left_hand_back: source[6] >= 0.5,
            left_hand_left: source[7] >= 0.5,
            left_hand_right: source[8] >= 0.5,
            right_foot_left: source[9] >= 0.5,
            right_foot_right: source[10] >= 0.5,
            right_hand_back: source[11] >= 0.5,
            right_hand_left: source[12] >= 0.5,
            right_hand_right: source[13] >= 0.5,
        }
    }
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct SonarSensors {
    left: f32,
    right: f32,
}

impl From<[f32; 2]> for SonarSensors {
    fn from(source: [f32; 2]) -> Self {
        Self {
            left: source[0],
            right: source[1],
        }
    }
}

#[derive(Debug, Default)]
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

impl From<[f32; 25]> for JointsArray {
    fn from(source: [f32; 25]) -> Self {
        Self {
            head_yaw: source[0],
            head_pitch: source[1],
            left_shoulder_pitch: source[2],
            left_shoulder_roll: source[3],
            left_elbow_yaw: source[4],
            left_elbow_roll: source[5],
            left_wrist_yaw: source[6],
            left_hip_yaw_pitch: source[7],
            left_hip_roll: source[8],
            left_hip_pitch: source[9],
            left_knee_pitch: source[10],
            left_ankle_pitch: source[11],
            left_ankle_roll: source[12],
            right_hip_roll: source[13],
            right_hip_pitch: source[14],
            right_knee_pitch: source[15],
            right_ankle_pitch: source[16],
            right_ankle_roll: source[17],
            right_shoulder_pitch: source[18],
            right_shoulder_roll: source[19],
            right_elbow_yaw: source[20],
            right_elbow_roll: source[21],
            right_wrist_yaw: source[22],
            left_hand: source[23],
            right_hand: source[24],
        }
    }
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct StateStorage {
    /// Seconds since proxy start
    received_at: f32,
    robot_configuration: RobotConfiguration,
    inertial_measurement_unit: InertialMeasurementUnit,
    force_sensitive_resistors: ForceSensitiveResistors,
    touch_sensors: TouchSensors,
    sonar_sensors: SonarSensors,
    position: JointsArray,
    stiffness: JointsArray,
    current: JointsArray,
    temperature: JointsArray,
    status: JointsArray,
}

impl StateStorage {
    pub fn from(received_at: Duration, state_message: &LoLAStateMessage) -> Self {
        Self {
            received_at: received_at.as_secs_f32(),
            robot_configuration: state_message.robot_configuration.into(),
            inertial_measurement_unit: state_message.into(),
            force_sensitive_resistors: state_message.force_sensitive_resistors.into(),
            touch_sensors: state_message.touch.into(),
            sonar_sensors: state_message.sonar.into(),
            position: state_message.position.into(),
            stiffness: state_message.stiffness.into(),
            current: state_message.current.into(),
            temperature: state_message.temperature.into(),
            status: state_message.status.into(),
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

#[derive(Debug, Default)]
#[repr(C)]
pub struct ControlStorage {
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

impl ControlStorage {
    pub fn fill_chest_into(&self, control_message: &mut LoLAControlMessage) {
        control_message.chest[0] = self.chest.red;
        control_message.chest[1] = self.chest.green;
        control_message.chest[2] = self.chest.blue;
    }

    pub fn fill_ears_into(&self, control_message: &mut LoLAControlMessage) {
        control_message.left_ear[0] = self.right_ear.intensity_at_0;
        control_message.left_ear[1] = self.right_ear.intensity_at_36;
        control_message.left_ear[2] = self.right_ear.intensity_at_72;
        control_message.left_ear[3] = self.right_ear.intensity_at_108;
        control_message.left_ear[4] = self.right_ear.intensity_at_144;
        control_message.left_ear[5] = self.right_ear.intensity_at_180;
        control_message.left_ear[6] = self.right_ear.intensity_at_216;
        control_message.left_ear[7] = self.right_ear.intensity_at_252;
        control_message.left_ear[8] = self.right_ear.intensity_at_288;
        control_message.left_ear[9] = self.right_ear.intensity_at_324;

        control_message.right_ear[0] = self.right_ear.intensity_at_324;
        control_message.right_ear[1] = self.right_ear.intensity_at_288;
        control_message.right_ear[2] = self.right_ear.intensity_at_252;
        control_message.right_ear[3] = self.right_ear.intensity_at_216;
        control_message.right_ear[4] = self.right_ear.intensity_at_180;
        control_message.right_ear[5] = self.right_ear.intensity_at_144;
        control_message.right_ear[6] = self.right_ear.intensity_at_108;
        control_message.right_ear[7] = self.right_ear.intensity_at_72;
        control_message.right_ear[8] = self.right_ear.intensity_at_36;
        control_message.right_ear[9] = self.right_ear.intensity_at_0;
    }

    pub fn fill_eyes_into(&self, control_message: &mut LoLAControlMessage) {
        control_message.left_eye[0] = self.left_eye.color_at_45.red;
        control_message.left_eye[1] = self.left_eye.color_at_0.red;
        control_message.left_eye[2] = self.left_eye.color_at_315.red;
        control_message.left_eye[3] = self.left_eye.color_at_270.red;
        control_message.left_eye[4] = self.left_eye.color_at_225.red;
        control_message.left_eye[5] = self.left_eye.color_at_180.red;
        control_message.left_eye[6] = self.left_eye.color_at_135.red;
        control_message.left_eye[7] = self.left_eye.color_at_90.red;

        control_message.left_eye[8] = self.left_eye.color_at_45.green;
        control_message.left_eye[9] = self.left_eye.color_at_0.green;
        control_message.left_eye[10] = self.left_eye.color_at_315.green;
        control_message.left_eye[11] = self.left_eye.color_at_270.green;
        control_message.left_eye[12] = self.left_eye.color_at_225.green;
        control_message.left_eye[13] = self.left_eye.color_at_180.green;
        control_message.left_eye[14] = self.left_eye.color_at_135.green;
        control_message.left_eye[15] = self.left_eye.color_at_90.green;

        control_message.left_eye[16] = self.left_eye.color_at_45.blue;
        control_message.left_eye[17] = self.left_eye.color_at_0.blue;
        control_message.left_eye[18] = self.left_eye.color_at_315.blue;
        control_message.left_eye[19] = self.left_eye.color_at_270.blue;
        control_message.left_eye[20] = self.left_eye.color_at_225.blue;
        control_message.left_eye[21] = self.left_eye.color_at_180.blue;
        control_message.left_eye[22] = self.left_eye.color_at_135.blue;
        control_message.left_eye[23] = self.left_eye.color_at_90.blue;

        control_message.right_eye[0] = self.right_eye.color_at_0.red;
        control_message.right_eye[1] = self.right_eye.color_at_45.red;
        control_message.right_eye[2] = self.right_eye.color_at_90.red;
        control_message.right_eye[3] = self.right_eye.color_at_135.red;
        control_message.right_eye[4] = self.right_eye.color_at_180.red;
        control_message.right_eye[5] = self.right_eye.color_at_225.red;
        control_message.right_eye[6] = self.right_eye.color_at_270.red;
        control_message.right_eye[7] = self.right_eye.color_at_315.red;

        control_message.right_eye[8] = self.right_eye.color_at_0.green;
        control_message.right_eye[9] = self.right_eye.color_at_45.green;
        control_message.right_eye[10] = self.right_eye.color_at_90.green;
        control_message.right_eye[11] = self.right_eye.color_at_135.green;
        control_message.right_eye[12] = self.right_eye.color_at_180.green;
        control_message.right_eye[13] = self.right_eye.color_at_225.green;
        control_message.right_eye[14] = self.right_eye.color_at_270.green;
        control_message.right_eye[15] = self.right_eye.color_at_315.green;

        control_message.right_eye[16] = self.right_eye.color_at_0.blue;
        control_message.right_eye[17] = self.right_eye.color_at_45.blue;
        control_message.right_eye[18] = self.right_eye.color_at_90.blue;
        control_message.right_eye[19] = self.right_eye.color_at_135.blue;
        control_message.right_eye[20] = self.right_eye.color_at_180.blue;
        control_message.right_eye[21] = self.right_eye.color_at_225.blue;
        control_message.right_eye[22] = self.right_eye.color_at_270.blue;
        control_message.right_eye[23] = self.right_eye.color_at_315.blue;
    }

    pub fn fill_foots_into(&self, control_message: &mut LoLAControlMessage) {
        control_message.left_foot[0] = self.left_foot.red;
        control_message.left_foot[1] = self.left_foot.green;
        control_message.left_foot[2] = self.left_foot.blue;

        control_message.right_foot[0] = self.right_foot.red;
        control_message.right_foot[1] = self.right_foot.green;
        control_message.right_foot[2] = self.right_foot.blue;
    }

    pub fn fill_position_into(&self, control_message: &mut LoLAControlMessage) {
        control_message.position[0] = self.position.head_yaw;
        control_message.position[1] = self.position.head_pitch;
        control_message.position[2] = self.position.left_shoulder_pitch;
        control_message.position[3] = self.position.left_shoulder_roll;
        control_message.position[4] = self.position.left_elbow_yaw;
        control_message.position[5] = self.position.left_elbow_roll;
        control_message.position[6] = self.position.left_wrist_yaw;
        control_message.position[7] = self.position.left_hip_yaw_pitch;
        control_message.position[8] = self.position.left_hip_roll;
        control_message.position[9] = self.position.left_hip_pitch;
        control_message.position[10] = self.position.left_knee_pitch;
        control_message.position[11] = self.position.left_ankle_pitch;
        control_message.position[12] = self.position.left_ankle_roll;
        control_message.position[13] = self.position.right_hip_roll;
        control_message.position[14] = self.position.right_hip_pitch;
        control_message.position[15] = self.position.right_knee_pitch;
        control_message.position[16] = self.position.right_ankle_pitch;
        control_message.position[17] = self.position.right_ankle_roll;
        control_message.position[18] = self.position.right_shoulder_pitch;
        control_message.position[19] = self.position.right_shoulder_roll;
        control_message.position[20] = self.position.right_elbow_yaw;
        control_message.position[21] = self.position.right_elbow_roll;
        control_message.position[22] = self.position.right_wrist_yaw;
        control_message.position[23] = self.position.left_hand;
        control_message.position[24] = self.position.right_hand;
    }

    pub fn fill_stiffness_into(&self, control_message: &mut LoLAControlMessage) {
        control_message.stiffness[0] = self.stiffness.head_yaw;
        control_message.stiffness[1] = self.stiffness.head_pitch;
        control_message.stiffness[2] = self.stiffness.left_shoulder_pitch;
        control_message.stiffness[3] = self.stiffness.left_shoulder_roll;
        control_message.stiffness[4] = self.stiffness.left_elbow_yaw;
        control_message.stiffness[5] = self.stiffness.left_elbow_roll;
        control_message.stiffness[6] = self.stiffness.left_wrist_yaw;
        control_message.stiffness[7] = self.stiffness.left_hip_yaw_pitch;
        control_message.stiffness[8] = self.stiffness.left_hip_roll;
        control_message.stiffness[9] = self.stiffness.left_hip_pitch;
        control_message.stiffness[10] = self.stiffness.left_knee_pitch;
        control_message.stiffness[11] = self.stiffness.left_ankle_pitch;
        control_message.stiffness[12] = self.stiffness.left_ankle_roll;
        control_message.stiffness[13] = self.stiffness.right_hip_roll;
        control_message.stiffness[14] = self.stiffness.right_hip_pitch;
        control_message.stiffness[15] = self.stiffness.right_knee_pitch;
        control_message.stiffness[16] = self.stiffness.right_ankle_pitch;
        control_message.stiffness[17] = self.stiffness.right_ankle_roll;
        control_message.stiffness[18] = self.stiffness.right_shoulder_pitch;
        control_message.stiffness[19] = self.stiffness.right_shoulder_roll;
        control_message.stiffness[20] = self.stiffness.right_elbow_yaw;
        control_message.stiffness[21] = self.stiffness.right_elbow_roll;
        control_message.stiffness[22] = self.stiffness.right_wrist_yaw;
        control_message.stiffness[23] = self.stiffness.left_hand;
        control_message.stiffness[24] = self.stiffness.right_hand;
    }
}

pub fn fill_red_eyes_into(seconds: &f64, control_message: &mut LoLAControlMessage) {
    let interval_from_0_to_1 = seconds - seconds.floor();
    let position = ((2.0 * PI * interval_from_0_to_1 as f32).sin() + 1.0) / 2.0;
    let maximal_distance_from_center = 1.0 / 4.0;

    //     1
    //  0     2
    // 7       3
    //  6     4
    //     5
    let led_positions_left = [
        0.7154822031355754,
        0.8333333333333334,
        0.9511844635310912,
        1.0,
        0.9511844635310912,
        0.8333333333333334,
        0.7154822031355754,
        0.6666666666666667,
    ];

    //     0
    //  1     7
    // 2       6
    //  3     5
    //     4
    let led_positions_right = [
        0.16666666666666666,
        0.04881553646890875,
        0.0,
        0.04881553646890875,
        0.16666666666666666,
        0.2845177968644246,
        0.3333333333333333,
        0.2845177968644246,
    ];

    let mut intensities_left = [0.0; 24];
    let mut intensities_right = [0.0; 24];

    for (intensity, led_position) in intensities_left.iter_mut().zip(led_positions_left.iter()) {
        let distance = (led_position - position).abs();
        *intensity =
            ((maximal_distance_from_center - distance) / maximal_distance_from_center).max(0.0);
    }
    for (intensity, led_position) in intensities_right.iter_mut().zip(led_positions_right.iter()) {
        let distance = (led_position - position).abs();
        *intensity =
            ((maximal_distance_from_center - distance) / maximal_distance_from_center).max(0.0);
    }

    control_message.left_eye = intensities_left;
    control_message.right_eye = intensities_right;
}
