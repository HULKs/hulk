use coordinate_systems::Transform;
use nalgebra::Isometry3;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::coordinate_systems::{
    Head, LeftAnkle, LeftElbow, LeftFoot, LeftForearm, LeftHip, LeftPelvis, LeftShoulder, LeftSole,
    LeftThigh, LeftTibia, LeftUpperArm, LeftWrist, Neck, RightAnkle, RightElbow, RightFoot,
    RightForearm, RightHip, RightPelvis, RightShoulder, RightSole, RightThigh, RightTibia,
    RightUpperArm, RightWrist, Robot, Torso,
};

#[derive(Debug, Clone, Default, SerializeHierarchy, Serialize, Deserialize)]
pub struct RobotKinematics {
    // head
    pub neck_to_robot: Transform<Neck, Robot, Isometry3<f32>>,
    pub head_to_robot: Transform<Head, Robot, Isometry3<f32>>,
    // torso
    pub torso_to_robot: Transform<Torso, Robot, Isometry3<f32>>,
    // left arm
    pub left_shoulder_to_robot: Transform<LeftShoulder, Robot, Isometry3<f32>>,
    pub left_upper_arm_to_robot: Transform<LeftUpperArm, Robot, Isometry3<f32>>,
    pub left_elbow_to_robot: Transform<LeftElbow, Robot, Isometry3<f32>>,
    pub left_forearm_to_robot: Transform<LeftForearm, Robot, Isometry3<f32>>,
    pub left_wrist_to_robot: Transform<LeftWrist, Robot, Isometry3<f32>>,
    // right arm
    pub right_shoulder_to_robot: Transform<RightShoulder, Robot, Isometry3<f32>>,
    pub right_upper_arm_to_robot: Transform<RightUpperArm, Robot, Isometry3<f32>>,
    pub right_elbow_to_robot: Transform<RightElbow, Robot, Isometry3<f32>>,
    pub right_forearm_to_robot: Transform<RightForearm, Robot, Isometry3<f32>>,
    pub right_wrist_to_robot: Transform<RightWrist, Robot, Isometry3<f32>>,
    // left leg
    pub left_pelvis_to_robot: Transform<LeftPelvis, Robot, Isometry3<f32>>,
    pub left_hip_to_robot: Transform<LeftHip, Robot, Isometry3<f32>>,
    pub left_thigh_to_robot: Transform<LeftThigh, Robot, Isometry3<f32>>,
    pub left_tibia_to_robot: Transform<LeftTibia, Robot, Isometry3<f32>>,
    pub left_ankle_to_robot: Transform<LeftAnkle, Robot, Isometry3<f32>>,
    pub left_foot_to_robot: Transform<LeftFoot, Robot, Isometry3<f32>>,
    pub left_sole_to_robot: Transform<LeftSole, Robot, Isometry3<f32>>,
    // right leg
    pub right_pelvis_to_robot: Transform<RightPelvis, Robot, Isometry3<f32>>,
    pub right_hip_to_robot: Transform<RightHip, Robot, Isometry3<f32>>,
    pub right_thigh_to_robot: Transform<RightThigh, Robot, Isometry3<f32>>,
    pub right_tibia_to_robot: Transform<RightTibia, Robot, Isometry3<f32>>,
    pub right_ankle_to_robot: Transform<RightAnkle, Robot, Isometry3<f32>>,
    pub right_foot_to_robot: Transform<RightFoot, Robot, Isometry3<f32>>,
    pub right_sole_to_robot: Transform<RightSole, Robot, Isometry3<f32>>,
}
