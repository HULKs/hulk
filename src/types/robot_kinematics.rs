use macros::SerializeHierarchy;
use nalgebra::Isometry3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, SerializeHierarchy, Serialize, Deserialize)]
pub struct RobotKinematics {
    // head
    pub neck_to_robot: Isometry3<f32>,
    pub head_to_robot: Isometry3<f32>,
    // torso
    pub torso_to_robot: Isometry3<f32>,
    // left arm
    pub left_shoulder_to_robot: Isometry3<f32>,
    pub left_upper_arm_to_robot: Isometry3<f32>,
    pub left_elbow_to_robot: Isometry3<f32>,
    pub left_forearm_to_robot: Isometry3<f32>,
    pub left_wrist_to_robot: Isometry3<f32>,
    // right arm
    pub right_shoulder_to_robot: Isometry3<f32>,
    pub right_upper_arm_to_robot: Isometry3<f32>,
    pub right_elbow_to_robot: Isometry3<f32>,
    pub right_forearm_to_robot: Isometry3<f32>,
    pub right_wrist_to_robot: Isometry3<f32>,
    // left leg
    pub left_pelvis_to_robot: Isometry3<f32>,
    pub left_hip_to_robot: Isometry3<f32>,
    pub left_thigh_to_robot: Isometry3<f32>,
    pub left_tibia_to_robot: Isometry3<f32>,
    pub left_ankle_to_robot: Isometry3<f32>,
    pub left_foot_to_robot: Isometry3<f32>,
    pub left_sole_to_robot: Isometry3<f32>,
    // right leg
    pub right_pelvis_to_robot: Isometry3<f32>,
    pub right_hip_to_robot: Isometry3<f32>,
    pub right_thigh_to_robot: Isometry3<f32>,
    pub right_tibia_to_robot: Isometry3<f32>,
    pub right_ankle_to_robot: Isometry3<f32>,
    pub right_foot_to_robot: Isometry3<f32>,
    pub right_sole_to_robot: Isometry3<f32>,
}
