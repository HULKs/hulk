use std::f32::consts::PI;

use coordinate_systems::{IntoTransform, Transform};
use nalgebra::{Isometry3, Translation, Vector3};
use types::{
    coordinate_systems::{
        Head, LeftAnkle, LeftElbow, LeftFoot, LeftForearm, LeftHip, LeftPelvis, LeftShoulder,
        LeftThigh, LeftTibia, LeftUpperArm, LeftWrist, Neck, RightAnkle, RightElbow, RightFoot,
        RightForearm, RightHip, RightPelvis, RightShoulder, RightThigh, RightTibia, RightUpperArm,
        RightWrist, Robot,
    },
    joints::{arm::ArmJoints, head::HeadJoints, leg::LegJoints},
    robot_dimensions::RobotDimensions,
};

pub fn neck_to_robot(angles: &HeadJoints<f32>) -> Transform<Neck, Robot, Isometry3<f32>> {
    (Translation::from(RobotDimensions::ROBOT_TO_NECK)
        * Isometry3::rotation(Vector3::z() * angles.yaw))
    .framed_transform()
}

pub fn head_to_neck(angles: &HeadJoints<f32>) -> Transform<Head, Neck, Isometry3<f32>> {
    Isometry3::rotation(Vector3::y() * angles.pitch).framed_transform()
}

// left arm
pub fn left_shoulder_to_robot(
    angles: &ArmJoints<f32>,
) -> Transform<LeftShoulder, Robot, Isometry3<f32>> {
    (Translation::from(RobotDimensions::ROBOT_TO_LEFT_SHOULDER)
        * Isometry3::rotation(Vector3::y() * angles.shoulder_pitch))
    .framed_transform()
}

pub fn left_upper_arm_to_left_shoulder(
    angles: &ArmJoints<f32>,
) -> Transform<LeftUpperArm, LeftShoulder, Isometry3<f32>> {
    (Isometry3::rotation(Vector3::z() * angles.shoulder_roll)).framed_transform()
}

pub fn left_elbow_to_left_upper_arm(
    angles: &ArmJoints<f32>,
) -> Transform<LeftElbow, LeftUpperArm, Isometry3<f32>> {
    (Translation::from(RobotDimensions::LEFT_SHOULDER_TO_LEFT_ELBOW)
        * Isometry3::rotation(Vector3::x() * angles.elbow_yaw))
    .framed_transform()
}

pub fn left_forearm_to_left_elbow(
    angles: &ArmJoints<f32>,
) -> Transform<LeftForearm, LeftElbow, Isometry3<f32>> {
    (Isometry3::rotation(Vector3::z() * angles.elbow_roll)).framed_transform()
}

pub fn left_wrist_to_left_forearm(
    angles: &ArmJoints<f32>,
) -> Transform<LeftWrist, LeftForearm, Isometry3<f32>> {
    (Translation::from(RobotDimensions::ELBOW_TO_WRIST)
        * Isometry3::rotation(Vector3::x() * angles.wrist_yaw))
    .framed_transform()
}

// right arm
pub fn right_shoulder_to_robot(
    angles: &ArmJoints<f32>,
) -> Transform<RightShoulder, Robot, Isometry3<f32>> {
    (Translation::from(RobotDimensions::ROBOT_TO_RIGHT_SHOULDER)
        * Isometry3::rotation(Vector3::y() * angles.shoulder_pitch))
    .framed_transform()
}

pub fn right_upper_arm_to_right_shoulder(
    angles: &ArmJoints<f32>,
) -> Transform<RightUpperArm, RightShoulder, Isometry3<f32>> {
    (Isometry3::rotation(Vector3::z() * angles.shoulder_roll)).framed_transform()
}

pub fn right_elbow_to_right_upper_arm(
    angles: &ArmJoints<f32>,
) -> Transform<RightElbow, RightUpperArm, Isometry3<f32>> {
    (Translation::from(RobotDimensions::RIGHT_SHOULDER_TO_RIGHT_ELBOW)
        * Isometry3::rotation(Vector3::x() * angles.elbow_yaw))
    .framed_transform()
}

pub fn right_forearm_to_right_elbow(
    angles: &ArmJoints<f32>,
) -> Transform<RightForearm, RightElbow, Isometry3<f32>> {
    (Isometry3::rotation(Vector3::z() * angles.elbow_roll)).framed_transform()
}

pub fn right_wrist_to_right_forearm(
    angles: &ArmJoints<f32>,
) -> Transform<RightWrist, RightForearm, Isometry3<f32>> {
    (Translation::from(RobotDimensions::ELBOW_TO_WRIST)
        * Isometry3::rotation(Vector3::x() * angles.wrist_yaw))
    .framed_transform()
}

// left leg
pub fn left_pelvis_to_robot(
    angles: &LegJoints<f32>,
) -> Transform<LeftPelvis, Robot, Isometry3<f32>> {
    (Translation::from(RobotDimensions::ROBOT_TO_LEFT_PELVIS)
        * Isometry3::rotation(Vector3::x() * PI / 4.0)
        * Isometry3::rotation(Vector3::z() * -angles.hip_yaw_pitch)
        * Isometry3::rotation(Vector3::x() * -PI / 4.0))
    .framed_transform()
}

pub fn left_hip_to_left_pelvis(
    angles: &LegJoints<f32>,
) -> Transform<LeftHip, LeftPelvis, Isometry3<f32>> {
    (Isometry3::rotation(Vector3::x() * angles.hip_roll)).framed_transform()
}

pub fn left_thigh_to_left_hip(
    angles: &LegJoints<f32>,
) -> Transform<LeftThigh, LeftHip, Isometry3<f32>> {
    (Isometry3::rotation(Vector3::y() * angles.hip_pitch)).framed_transform()
}

pub fn left_tibia_to_left_thigh(
    angles: &LegJoints<f32>,
) -> Transform<LeftTibia, LeftThigh, Isometry3<f32>> {
    (Translation::from(RobotDimensions::HIP_TO_KNEE)
        * Isometry3::rotation(Vector3::y() * angles.knee_pitch))
    .framed_transform()
}

pub fn left_ankle_to_left_tibia(
    angles: &LegJoints<f32>,
) -> Transform<LeftAnkle, LeftTibia, Isometry3<f32>> {
    (Translation::from(RobotDimensions::KNEE_TO_ANKLE)
        * Isometry3::rotation(Vector3::y() * angles.ankle_pitch))
    .framed_transform()
}

pub fn left_foot_to_left_ankle(
    angles: &LegJoints<f32>,
) -> Transform<LeftFoot, LeftAnkle, Isometry3<f32>> {
    (Isometry3::rotation(Vector3::x() * angles.ankle_roll)).framed_transform()
}

// right leg
pub fn right_pelvis_to_robot(
    angles: &LegJoints<f32>,
) -> Transform<RightPelvis, Robot, Isometry3<f32>> {
    (Translation::from(RobotDimensions::ROBOT_TO_RIGHT_PELVIS)
        * Isometry3::rotation(Vector3::x() * -PI / 4.0)
        * Isometry3::rotation(Vector3::z() * angles.hip_yaw_pitch)
        * Isometry3::rotation(Vector3::x() * PI / 4.0))
    .framed_transform()
}

pub fn right_hip_to_right_pelvis(
    angles: &LegJoints<f32>,
) -> Transform<RightHip, RightPelvis, Isometry3<f32>> {
    (Isometry3::rotation(Vector3::x() * angles.hip_roll)).framed_transform()
}

pub fn right_thigh_to_right_hip(
    angles: &LegJoints<f32>,
) -> Transform<RightThigh, RightHip, Isometry3<f32>> {
    (Isometry3::rotation(Vector3::y() * angles.hip_pitch)).framed_transform()
}

pub fn right_tibia_to_right_thigh(
    angles: &LegJoints<f32>,
) -> Transform<RightTibia, RightThigh, Isometry3<f32>> {
    (Translation::from(RobotDimensions::HIP_TO_KNEE)
        * Isometry3::rotation(Vector3::y() * angles.knee_pitch))
    .framed_transform()
}

pub fn right_ankle_to_right_tibia(
    angles: &LegJoints<f32>,
) -> Transform<RightAnkle, RightTibia, Isometry3<f32>> {
    (Translation::from(RobotDimensions::KNEE_TO_ANKLE)
        * Isometry3::rotation(Vector3::y() * angles.ankle_pitch))
    .framed_transform()
}

pub fn right_foot_to_right_ankle(
    angles: &LegJoints<f32>,
) -> Transform<RightFoot, RightAnkle, Isometry3<f32>> {
    (Isometry3::rotation(Vector3::x() * angles.ankle_roll)).framed_transform()
}
