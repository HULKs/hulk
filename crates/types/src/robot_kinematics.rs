use serde::{Deserialize, Serialize};

use linear_algebra::Isometry3;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use coordinate_systems::{
    Head, LeftAnkle, LeftElbow, LeftFoot, LeftForearm, LeftHip, LeftPelvis, LeftShoulder, LeftSole,
    LeftThigh, LeftTibia, LeftUpperArm, LeftWrist, Neck, RightAnkle, RightElbow, RightFoot,
    RightForearm, RightHip, RightPelvis, RightShoulder, RightSole, RightThigh, RightTibia,
    RightUpperArm, RightWrist, Robot, Torso,
};

#[derive(
    Debug, Clone, Default, PathSerialize, PathDeserialize, PathIntrospect, Serialize, Deserialize,
)]
pub struct RobotHeadKinematics {
    pub neck_to_robot: Isometry3<Neck, Robot>,
    pub head_to_robot: Isometry3<Head, Robot>,
}

#[derive(
    Debug, Clone, Default, PathSerialize, PathDeserialize, PathIntrospect, Serialize, Deserialize,
)]
pub struct RobotTorsoKinematics {
    pub torso_to_robot: Isometry3<Torso, Robot>,
}

#[derive(
    Debug, Clone, Default, PathSerialize, PathDeserialize, PathIntrospect, Serialize, Deserialize,
)]
pub struct RobotLeftArmKinematics {
    pub shoulder_to_robot: Isometry3<LeftShoulder, Robot>,
    pub upper_arm_to_robot: Isometry3<LeftUpperArm, Robot>,
    pub elbow_to_robot: Isometry3<LeftElbow, Robot>,
    pub forearm_to_robot: Isometry3<LeftForearm, Robot>,
    pub wrist_to_robot: Isometry3<LeftWrist, Robot>,
}

#[derive(
    Debug, Clone, Default, PathSerialize, PathDeserialize, PathIntrospect, Serialize, Deserialize,
)]
pub struct RobotRightArmKinematics {
    pub shoulder_to_robot: Isometry3<RightShoulder, Robot>,
    pub upper_arm_to_robot: Isometry3<RightUpperArm, Robot>,
    pub elbow_to_robot: Isometry3<RightElbow, Robot>,
    pub forearm_to_robot: Isometry3<RightForearm, Robot>,
    pub wrist_to_robot: Isometry3<RightWrist, Robot>,
}

#[derive(
    Debug, Clone, Default, PathSerialize, PathDeserialize, PathIntrospect, Serialize, Deserialize,
)]
pub struct RobotLeftLegKinematics {
    pub pelvis_to_robot: Isometry3<LeftPelvis, Robot>,
    pub hip_to_robot: Isometry3<LeftHip, Robot>,
    pub thigh_to_robot: Isometry3<LeftThigh, Robot>,
    pub tibia_to_robot: Isometry3<LeftTibia, Robot>,
    pub ankle_to_robot: Isometry3<LeftAnkle, Robot>,
    pub foot_to_robot: Isometry3<LeftFoot, Robot>,
    pub sole_to_robot: Isometry3<LeftSole, Robot>,
}

#[derive(
    Debug, Clone, Default, PathSerialize, PathDeserialize, PathIntrospect, Serialize, Deserialize,
)]
pub struct RobotRightLegKinematics {
    pub pelvis_to_robot: Isometry3<RightPelvis, Robot>,
    pub hip_to_robot: Isometry3<RightHip, Robot>,
    pub thigh_to_robot: Isometry3<RightThigh, Robot>,
    pub tibia_to_robot: Isometry3<RightTibia, Robot>,
    pub ankle_to_robot: Isometry3<RightAnkle, Robot>,
    pub foot_to_robot: Isometry3<RightFoot, Robot>,
    pub sole_to_robot: Isometry3<RightSole, Robot>,
}

#[derive(
    Debug, Clone, Default, PathSerialize, PathDeserialize, PathIntrospect, Serialize, Deserialize,
)]
pub struct RobotKinematics {
    pub head: RobotHeadKinematics,
    pub torso: RobotTorsoKinematics,
    pub left_arm: RobotLeftArmKinematics,
    pub right_arm: RobotRightArmKinematics,
    pub left_leg: RobotLeftLegKinematics,
    pub right_leg: RobotRightLegKinematics,
}
