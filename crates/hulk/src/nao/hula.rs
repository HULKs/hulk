use std::{io::Write, mem::size_of, os::unix::net::UnixStream, slice::from_raw_parts};

use color_eyre::{eyre::Context, Result};
use nalgebra::{vector, Vector2, Vector3};
use types::{self, ArmJoints, HeadJoints, Joints, LegJoints};

use super::double_buffered_reader::{DoubleBufferedReader, SelectPoller};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct RobotConfiguration {
    pub body_id: [u8; 20],
    pub body_version: u8,
    pub head_id: [u8; 20],
    pub head_version: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Battery {
    pub charge: f32,
    pub status: f32,
    pub current: f32,
    pub temperature: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Vertex2 {
    x: f32,
    y: f32,
}

impl From<Vertex2> for Vector2<f32> {
    fn from(vertex: Vertex2) -> Self {
        vector![vertex.x, vertex.y]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Vertex3 {
    x: f32,
    y: f32,
    z: f32,
}

impl From<Vertex3> for Vector3<f32> {
    fn from(vertex: Vertex3) -> Self {
        vector![vertex.x, vertex.y, vertex.z]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct InertialMeasurementUnit {
    pub accelerometer: Vertex3,
    pub angles: Vertex2,
    pub gyroscope: Vertex3,
}

impl From<InertialMeasurementUnit> for types::InertialMeasurementUnitData {
    fn from(from: InertialMeasurementUnit) -> Self {
        types::InertialMeasurementUnitData {
            linear_acceleration: -Vector3::from(from.accelerometer),
            angular_velocity: from.gyroscope.into(),
            roll_pitch: from.angles.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

impl From<ForceSensitiveResistors> for types::ForceSensitiveResistors {
    fn from(from: ForceSensitiveResistors) -> Self {
        types::ForceSensitiveResistors {
            left: types::Foot {
                front_left: from.left_foot_front_left,
                front_right: from.left_foot_front_right,
                rear_left: from.left_foot_rear_left,
                rear_right: from.left_foot_rear_right,
            },
            right: types::Foot {
                front_left: from.right_foot_front_left,
                front_right: from.right_foot_front_right,
                rear_left: from.right_foot_rear_left,
                rear_right: from.right_foot_rear_right,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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

impl From<TouchSensors> for types::TouchSensors {
    fn from(from: TouchSensors) -> Self {
        types::TouchSensors {
            chest_button: from.chest_button,
            head_front: from.head_front,
            head_middle: from.head_middle,
            head_rear: from.head_rear,
            left_foot_left: from.left_foot_left,
            left_foot_right: from.left_foot_right,
            left_hand_back: from.left_hand_back,
            left_hand_left: from.left_hand_left,
            left_hand_right: from.left_hand_right,
            right_foot_left: from.right_foot_left,
            right_foot_right: from.right_foot_right,
            right_hand_back: from.right_hand_back,
            right_hand_left: from.right_hand_left,
            right_hand_right: from.right_hand_right,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct SonarSensors {
    pub left: f32,
    pub right: f32,
}

impl From<SonarSensors> for types::SonarSensors {
    fn from(from: SonarSensors) -> Self {
        types::SonarSensors {
            left: from.left,
            right: from.right,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct JointsArray {
    pub head_yaw: f32,
    pub head_pitch: f32,
    pub left_shoulder_pitch: f32,
    pub left_shoulder_roll: f32,
    pub left_elbow_yaw: f32,
    pub left_elbow_roll: f32,
    pub left_wrist_yaw: f32,
    pub left_hip_yaw_pitch: f32,
    pub left_hip_roll: f32,
    pub left_hip_pitch: f32,
    pub left_knee_pitch: f32,
    pub left_ankle_pitch: f32,
    pub left_ankle_roll: f32,
    // pub right_hip_yaw_pitch: f32, TODO
    pub right_hip_roll: f32,
    pub right_hip_pitch: f32,
    pub right_knee_pitch: f32,
    pub right_ankle_pitch: f32,
    pub right_ankle_roll: f32,
    pub right_shoulder_pitch: f32,
    pub right_shoulder_roll: f32,
    pub right_elbow_yaw: f32,
    pub right_elbow_roll: f32,
    pub right_wrist_yaw: f32,
    pub left_hand: f32,
    pub right_hand: f32,
}

impl From<Joints> for JointsArray {
    fn from(joints: Joints) -> Self {
        Self {
            head_yaw: joints.head.yaw,
            head_pitch: joints.head.pitch,
            left_shoulder_pitch: joints.left_arm.shoulder_pitch,
            left_shoulder_roll: joints.left_arm.shoulder_roll,
            left_elbow_yaw: joints.left_arm.elbow_yaw,
            left_elbow_roll: joints.left_arm.elbow_roll,
            left_wrist_yaw: joints.left_arm.wrist_yaw,
            left_hip_yaw_pitch: joints.left_leg.hip_yaw_pitch,
            left_hip_roll: joints.left_leg.hip_roll,
            left_hip_pitch: joints.left_leg.hip_pitch,
            left_knee_pitch: joints.left_leg.knee_pitch,
            left_ankle_pitch: joints.left_leg.ankle_pitch,
            left_ankle_roll: joints.left_leg.ankle_roll,
            right_hip_roll: joints.right_leg.hip_roll,
            right_hip_pitch: joints.right_leg.hip_pitch,
            right_knee_pitch: joints.right_leg.knee_pitch,
            right_ankle_pitch: joints.right_leg.ankle_pitch,
            right_ankle_roll: joints.right_leg.ankle_roll,
            right_shoulder_pitch: joints.right_arm.shoulder_pitch,
            right_shoulder_roll: joints.right_arm.shoulder_roll,
            right_elbow_yaw: joints.right_arm.elbow_yaw,
            right_elbow_roll: joints.right_arm.elbow_roll,
            right_wrist_yaw: joints.right_arm.wrist_yaw,
            left_hand: joints.left_arm.hand,
            right_hand: joints.right_arm.hand,
        }
    }
}

impl From<JointsArray> for Joints {
    fn from(joints: JointsArray) -> Self {
        Joints {
            head: HeadJoints {
                yaw: joints.head_yaw,
                pitch: joints.head_pitch,
            },
            left_arm: ArmJoints {
                shoulder_pitch: joints.left_shoulder_pitch,
                shoulder_roll: joints.left_shoulder_roll,
                elbow_yaw: joints.left_elbow_yaw,
                elbow_roll: joints.left_elbow_roll,
                wrist_yaw: joints.left_wrist_yaw,
                hand: joints.left_hand,
            },
            right_arm: ArmJoints {
                shoulder_pitch: joints.right_shoulder_pitch,
                shoulder_roll: joints.right_shoulder_roll,
                elbow_yaw: joints.right_elbow_yaw,
                elbow_roll: joints.right_elbow_roll,
                wrist_yaw: joints.right_wrist_yaw,
                hand: joints.right_hand,
            },
            left_leg: LegJoints {
                hip_yaw_pitch: joints.left_hip_yaw_pitch,
                hip_roll: joints.left_hip_roll,
                hip_pitch: joints.left_hip_pitch,
                knee_pitch: joints.left_knee_pitch,
                ankle_pitch: joints.left_ankle_pitch,
                ankle_roll: joints.left_ankle_roll,
            },
            right_leg: LegJoints {
                hip_yaw_pitch: joints.left_hip_yaw_pitch, //TODO
                hip_roll: joints.right_hip_roll,
                hip_pitch: joints.right_hip_pitch,
                knee_pitch: joints.right_knee_pitch,
                ankle_pitch: joints.right_ankle_pitch,
                ankle_roll: joints.right_ankle_roll,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct StateStorage {
    /// Seconds since proxy start
    pub received_at: f32,
    pub robot_configuration: RobotConfiguration,
    pub battery: Battery,
    pub inertial_measurement_unit: InertialMeasurementUnit,
    pub force_sensitive_resistors: ForceSensitiveResistors,
    pub touch_sensors: TouchSensors,
    pub sonar_sensors: SonarSensors,
    pub position: JointsArray,
    pub stiffness: JointsArray,
    pub current: JointsArray,
    pub temperature: JointsArray,
    pub status: JointsArray,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

impl From<types::Rgb> for Color {
    fn from(color: types::Rgb) -> Self {
        Self {
            red: color.r as f32 / 255.0,
            green: color.g as f32 / 255.0,
            blue: color.b as f32 / 255.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Eye {
    pub color_at_0: Color,
    pub color_at_45: Color,
    pub color_at_90: Color,
    pub color_at_135: Color,
    pub color_at_180: Color,
    pub color_at_225: Color,
    pub color_at_270: Color,
    pub color_at_315: Color,
}

impl From<types::Eye> for Eye {
    fn from(eye: types::Eye) -> Self {
        Self {
            color_at_0: eye.color_at_0.into(),
            color_at_45: eye.color_at_45.into(),
            color_at_90: eye.color_at_90.into(),
            color_at_135: eye.color_at_135.into(),
            color_at_180: eye.color_at_180.into(),
            color_at_225: eye.color_at_225.into(),
            color_at_270: eye.color_at_270.into(),
            color_at_315: eye.color_at_315.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Ear {
    pub intensity_at_0: f32,
    pub intensity_at_36: f32,
    pub intensity_at_72: f32,
    pub intensity_at_108: f32,
    pub intensity_at_144: f32,
    pub intensity_at_180: f32,
    pub intensity_at_216: f32,
    pub intensity_at_252: f32,
    pub intensity_at_288: f32,
    pub intensity_at_324: f32,
}

impl From<types::Ear> for Ear {
    fn from(ear: types::Ear) -> Self {
        Self {
            intensity_at_0: ear.intensity_at_0,
            intensity_at_36: ear.intensity_at_36,
            intensity_at_72: ear.intensity_at_72,
            intensity_at_108: ear.intensity_at_108,
            intensity_at_144: ear.intensity_at_144,
            intensity_at_180: ear.intensity_at_180,
            intensity_at_216: ear.intensity_at_216,
            intensity_at_252: ear.intensity_at_252,
            intensity_at_288: ear.intensity_at_288,
            intensity_at_324: ear.intensity_at_324,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct ControlStorage {
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

pub fn read_from_hula(
    reader: &mut DoubleBufferedReader<StateStorage, UnixStream, SelectPoller>,
) -> Result<StateStorage> {
    Ok(*reader
        .draining_read()
        .wrap_err("failed to drain from stream")?)
}

pub fn write_to_hula(stream: &mut UnixStream, control_storage: ControlStorage) -> Result<()> {
    let control_storage_buffer = unsafe {
        from_raw_parts(
            &control_storage as *const ControlStorage as *const u8,
            size_of::<ControlStorage>(),
        )
    };
    stream.write_all(control_storage_buffer)?;
    stream.flush()?;
    Ok(())
}
