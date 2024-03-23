use linear_algebra::{vector, Vector3};

use coordinate_systems::{
    Head, LeftFoot, LeftForearm, LeftThigh, LeftTibia, LeftUpperArm, RightFoot, RightForearm,
    RightThigh, RightTibia, RightUpperArm, Robot,
};

#[derive(Debug)]
pub struct RobotDimensions {}

impl RobotDimensions {
    pub const ROBOT_TO_TORSO: Vector3<Robot> = vector![-0.00413, 0.0, 0.12842];
    pub const ROBOT_TO_NECK: Vector3<Robot> = vector![0.0, 0.0, 0.2115];
    pub const ROBOT_TO_LEFT_PELVIS: Vector3<Robot> = vector![0.0, 0.05, 0.0];
    pub const ROBOT_TO_RIGHT_PELVIS: Vector3<Robot> = vector![0.0, -0.05, 0.0];
    pub const LEFT_HIP_TO_LEFT_KNEE: Vector3<LeftThigh> = vector![0.0, 0.0, -0.1];
    pub const RIGHT_HIP_TO_RIGHT_KNEE: Vector3<RightThigh> = vector![0.0, 0.0, -0.1];
    pub const LEFT_KNEE_TO_LEFT_ANKLE: Vector3<LeftTibia> = vector![0.0, 0.0, -0.1029];
    pub const RIGHT_KNEE_TO_RIGHT_ANKLE: Vector3<RightTibia> = vector![0.0, 0.0, -0.1029];
    pub const LEFT_ANKLE_TO_LEFT_SOLE: Vector3<LeftFoot> = vector![0.0, 0.0, -0.04519];
    pub const RIGHT_ANKLE_TO_RIGHT_SOLE: Vector3<RightFoot> = vector![0.0, 0.0, -0.04519];
    pub const ROBOT_TO_LEFT_SHOULDER: Vector3<Robot> = vector![0.0, 0.098, 0.185];
    pub const ROBOT_TO_RIGHT_SHOULDER: Vector3<Robot> = vector![0.0, -0.098, 0.185];
    pub const LEFT_SHOULDER_TO_LEFT_ELBOW: Vector3<LeftUpperArm> = vector![0.105, 0.015, 0.0];
    pub const RIGHT_SHOULDER_TO_RIGHT_ELBOW: Vector3<RightUpperArm> = vector![0.105, -0.015, 0.0];
    pub const LEFT_ELBOW_TO_LEFT_WRIST: Vector3<LeftForearm> = vector![0.05595, 0.0, 0.0];
    pub const RIGHT_ELBOW_TO_RIGHT_WRIST: Vector3<RightForearm> = vector![0.05595, 0.0, 0.0];
    pub const HEAD_TO_TOP_CAMERA: Vector3<Head> = vector![0.05871, 0.0, 0.06364];
    pub const HEAD_TO_BOTTOM_CAMERA: Vector3<Head> = vector![0.05071, 0.0, 0.01774];
}
