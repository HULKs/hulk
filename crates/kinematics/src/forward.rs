use std::f32::consts::PI;

use coordinate_systems::{
    Head, LeftAnkle, LeftElbow, LeftFoot, LeftForearm, LeftHip, LeftPelvis, LeftShoulder,
    LeftThigh, LeftTibia, LeftUpperArm, LeftWrist, Neck, RightAnkle, RightElbow, RightFoot,
    RightForearm, RightHip, RightPelvis, RightShoulder, RightThigh, RightTibia, RightUpperArm,
    RightWrist, Robot,
};
use linear_algebra::{IntoFramed, Isometry3, Orientation3, Vector3};
use types::{
    joints::{arm::ArmJoints, head::HeadJoints, leg::LegJoints},
    robot_dimensions::RobotDimensions,
};

pub fn neck_to_robot(angles: &HeadJoints<f32>) -> Isometry3<Neck, Robot> {
    Isometry3::from_parts(
        RobotDimensions::ROBOT_TO_NECK,
        Orientation3::new(Vector3::z_axis() * angles.yaw),
    )
}

pub fn head_to_neck(angles: &HeadJoints<f32>) -> Isometry3<Head, Neck> {
    Isometry3::rotation(Vector3::y_axis() * angles.pitch)
}

// left arm
pub fn left_shoulder_to_robot(angles: &ArmJoints<f32>) -> Isometry3<LeftShoulder, Robot> {
    Isometry3::from_parts(
        RobotDimensions::ROBOT_TO_LEFT_SHOULDER,
        Orientation3::new(Vector3::y_axis() * angles.shoulder_pitch),
    )
}

pub fn left_upper_arm_to_left_shoulder(
    angles: &ArmJoints<f32>,
) -> Isometry3<LeftUpperArm, LeftShoulder> {
    Isometry3::rotation(Vector3::z_axis() * angles.shoulder_roll)
}

pub fn left_elbow_to_left_upper_arm(angles: &ArmJoints<f32>) -> Isometry3<LeftElbow, LeftUpperArm> {
    Isometry3::from_parts(
        RobotDimensions::LEFT_SHOULDER_TO_LEFT_ELBOW,
        Orientation3::new(Vector3::x_axis() * angles.elbow_yaw),
    )
}

pub fn left_forearm_to_left_elbow(angles: &ArmJoints<f32>) -> Isometry3<LeftForearm, LeftElbow> {
    Isometry3::rotation(Vector3::z_axis() * angles.elbow_roll)
}

pub fn left_wrist_to_left_forearm(angles: &ArmJoints<f32>) -> Isometry3<LeftWrist, LeftForearm> {
    Isometry3::from_parts(
        RobotDimensions::LEFT_ELBOW_TO_LEFT_WRIST,
        Orientation3::new(Vector3::x_axis() * angles.wrist_yaw),
    )
}

// right arm
pub fn right_shoulder_to_robot(angles: &ArmJoints<f32>) -> Isometry3<RightShoulder, Robot> {
    Isometry3::from_parts(
        RobotDimensions::ROBOT_TO_RIGHT_SHOULDER,
        Orientation3::new(Vector3::y_axis() * angles.shoulder_pitch),
    )
}

pub fn right_upper_arm_to_right_shoulder(
    angles: &ArmJoints<f32>,
) -> Isometry3<RightUpperArm, RightShoulder> {
    Isometry3::rotation(Vector3::z_axis() * angles.shoulder_roll)
}

pub fn right_elbow_to_right_upper_arm(
    angles: &ArmJoints<f32>,
) -> Isometry3<RightElbow, RightUpperArm> {
    Isometry3::from_parts(
        RobotDimensions::RIGHT_SHOULDER_TO_RIGHT_ELBOW,
        Orientation3::new(Vector3::x_axis() * angles.elbow_yaw),
    )
}

pub fn right_forearm_to_right_elbow(
    angles: &ArmJoints<f32>,
) -> Isometry3<RightForearm, RightElbow> {
    Isometry3::rotation(Vector3::z_axis() * angles.elbow_roll)
}

pub fn right_wrist_to_right_forearm(
    angles: &ArmJoints<f32>,
) -> Isometry3<RightWrist, RightForearm> {
    Isometry3::from_parts(
        RobotDimensions::RIGHT_ELBOW_TO_RIGHT_WRIST,
        Orientation3::new(Vector3::x_axis() * angles.wrist_yaw),
    )
}

// left leg
pub fn left_pelvis_to_robot(angles: &LegJoints<f32>) -> Isometry3<LeftPelvis, Robot> {
    let rotation = nalgebra::UnitQuaternion::new(nalgebra::Vector3::x() * PI / 4.0)
        * nalgebra::UnitQuaternion::new(nalgebra::Vector3::z() * -angles.hip_yaw_pitch)
        * nalgebra::UnitQuaternion::new(nalgebra::Vector3::x() * -PI / 4.0);
    Isometry3::from_parts(RobotDimensions::ROBOT_TO_LEFT_PELVIS, rotation.framed())
}

pub fn left_hip_to_left_pelvis(angles: &LegJoints<f32>) -> Isometry3<LeftHip, LeftPelvis> {
    Isometry3::rotation(Vector3::x_axis() * angles.hip_roll)
}

pub fn left_thigh_to_left_hip(angles: &LegJoints<f32>) -> Isometry3<LeftThigh, LeftHip> {
    Isometry3::rotation(Vector3::y_axis() * angles.hip_pitch)
}

pub fn left_tibia_to_left_thigh(angles: &LegJoints<f32>) -> Isometry3<LeftTibia, LeftThigh> {
    Isometry3::from_parts(
        RobotDimensions::LEFT_HIP_TO_LEFT_KNEE,
        Orientation3::new(Vector3::y_axis() * angles.knee_pitch),
    )
}

pub fn left_ankle_to_left_tibia(angles: &LegJoints<f32>) -> Isometry3<LeftAnkle, LeftTibia> {
    Isometry3::from_parts(
        RobotDimensions::LEFT_KNEE_TO_LEFT_ANKLE,
        Orientation3::new(Vector3::y_axis() * angles.ankle_pitch),
    )
}

pub fn left_foot_to_left_ankle(angles: &LegJoints<f32>) -> Isometry3<LeftFoot, LeftAnkle> {
    Isometry3::rotation(Vector3::x_axis() * angles.ankle_roll)
}

// right leg
pub fn right_pelvis_to_robot(angles: &LegJoints<f32>) -> Isometry3<RightPelvis, Robot> {
    let rotation = nalgebra::UnitQuaternion::new(nalgebra::Vector3::x() * -PI / 4.0)
        * nalgebra::UnitQuaternion::new(nalgebra::Vector3::z() * angles.hip_yaw_pitch)
        * nalgebra::UnitQuaternion::new(nalgebra::Vector3::x() * PI / 4.0);
    Isometry3::from_parts(RobotDimensions::ROBOT_TO_RIGHT_PELVIS, rotation.framed())
}

pub fn right_hip_to_right_pelvis(angles: &LegJoints<f32>) -> Isometry3<RightHip, RightPelvis> {
    Isometry3::rotation(Vector3::x_axis() * angles.hip_roll)
}

pub fn right_thigh_to_right_hip(angles: &LegJoints<f32>) -> Isometry3<RightThigh, RightHip> {
    Isometry3::rotation(Vector3::y_axis() * angles.hip_pitch)
}

pub fn right_tibia_to_right_thigh(angles: &LegJoints<f32>) -> Isometry3<RightTibia, RightThigh> {
    Isometry3::from_parts(
        RobotDimensions::RIGHT_HIP_TO_RIGHT_KNEE,
        Orientation3::new(Vector3::y_axis() * angles.knee_pitch),
    )
}

pub fn right_ankle_to_right_tibia(angles: &LegJoints<f32>) -> Isometry3<RightAnkle, RightTibia> {
    Isometry3::from_parts(
        RobotDimensions::RIGHT_KNEE_TO_RIGHT_ANKLE,
        Orientation3::new(Vector3::y_axis() * angles.ankle_pitch),
    )
}

pub fn right_foot_to_right_ankle(angles: &LegJoints<f32>) -> Isometry3<RightFoot, RightAnkle> {
    Isometry3::rotation(Vector3::x_axis() * angles.ankle_roll)
}
