use serde::{Deserialize, Serialize};

use linear_algebra::Isometry3;
use serialize_hierarchy::SerializeHierarchy;

use coordinate_systems::{
    Head, LeftAnkle, LeftElbow, LeftFoot, LeftForearm, LeftHip, LeftPelvis, LeftShoulder, LeftSole,
    LeftThigh, LeftTibia, LeftUpperArm, LeftWrist, Neck, RightAnkle, RightElbow, RightFoot,
    RightForearm, RightHip, RightPelvis, RightShoulder, RightSole, RightThigh, RightTibia,
    RightUpperArm, RightWrist, Robot, Torso,
};

#[derive(Debug, Clone, Default, SerializeHierarchy, Serialize, Deserialize)]
pub struct RobotKinematics {
    // head
    pub neck_to_robot: Isometry3<Neck, Robot>,
    pub head_to_robot: Isometry3<Head, Robot>,
    // torso
    pub torso_to_robot: Isometry3<Torso, Robot>,
    // left arm
    pub left_shoulder_to_robot: Isometry3<LeftShoulder, Robot>,
    pub left_upper_arm_to_robot: Isometry3<LeftUpperArm, Robot>,
    pub left_elbow_to_robot: Isometry3<LeftElbow, Robot>,
    pub left_forearm_to_robot: Isometry3<LeftForearm, Robot>,
    pub left_wrist_to_robot: Isometry3<LeftWrist, Robot>,
    // right arm
    pub right_shoulder_to_robot: Isometry3<RightShoulder, Robot>,
    pub right_upper_arm_to_robot: Isometry3<RightUpperArm, Robot>,
    pub right_elbow_to_robot: Isometry3<RightElbow, Robot>,
    pub right_forearm_to_robot: Isometry3<RightForearm, Robot>,
    pub right_wrist_to_robot: Isometry3<RightWrist, Robot>,
    // left leg
    pub left_pelvis_to_robot: Isometry3<LeftPelvis, Robot>,
    pub left_hip_to_robot: Isometry3<LeftHip, Robot>,
    pub left_thigh_to_robot: Isometry3<LeftThigh, Robot>,
    pub left_tibia_to_robot: Isometry3<LeftTibia, Robot>,
    pub left_ankle_to_robot: Isometry3<LeftAnkle, Robot>,
    pub left_foot_to_robot: Isometry3<LeftFoot, Robot>,
    pub left_sole_to_robot: Isometry3<LeftSole, Robot>,
    // right leg
    pub right_pelvis_to_robot: Isometry3<RightPelvis, Robot>,
    pub right_hip_to_robot: Isometry3<RightHip, Robot>,
    pub right_thigh_to_robot: Isometry3<RightThigh, Robot>,
    pub right_tibia_to_robot: Isometry3<RightTibia, Robot>,
    pub right_ankle_to_robot: Isometry3<RightAnkle, Robot>,
    pub right_foot_to_robot: Isometry3<RightFoot, Robot>,
    pub right_sole_to_robot: Isometry3<RightSole, Robot>,
}
