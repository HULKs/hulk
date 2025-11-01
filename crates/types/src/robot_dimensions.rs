use linear_algebra::{vector, Vector3};

use coordinate_systems::{
    Head, LeftFoot, LeftHip, LeftInnerShoulder, LeftOuterShoulder, LeftPelvis, LeftThigh,
    LeftTibia, LeftUpperArm, Neck, RightFoot, RightHip, RightInnerShoulder, RightOuterShoulder,
    RightPelvis, RightThigh, RightTibia, RightUpperArm, Robot,
};

#[derive(Debug)]
pub struct RobotDimensions {}

impl RobotDimensions {
    pub const ROBOT_TO_TORSO: Vector3<Robot> = vector![0.0, 0.0, 0.0]; // TODO
    pub const ROBOT_TO_NECK: Vector3<Robot> = vector![0.0056, 0.0, 0.2149];
    pub const NECK_TO_HEAD: Vector3<Neck> = vector![0.0, 0.0, 0.033];

    pub const HEAD_TO_TOP_CAMERA: Vector3<Head> = vector![0.05871, 0.0, 0.06364]; // TODO
    pub const HEAD_TO_BOTTOM_CAMERA: Vector3<Head> = vector![0.05071, 0.0, 0.01774]; // TODO

    pub const ROBOT_TO_LEFT_INNER_SHOULDER: Vector3<Robot> = vector![0.0, 0.077, 0.1845];
    pub const LEFT_INNER_SHOULDER_TO_LEFT_OUTER_SHOULDER: Vector3<LeftInnerShoulder> =
        vector![0.0025, 0.068, -0.0135];
    pub const LEFT_OUTER_SHOULDER_TO_LEFT_UPPER_ARM: Vector3<LeftOuterShoulder> =
        vector![0.0, 0.044428, 0.0];
    pub const LEFT_UPPER_ARM_TO_LEFT_FOREARM: Vector3<LeftUpperArm> = vector![0.0, 0.1215, 0.0];

    pub const ROBOT_TO_RIGHT_INNER_SHOULDER: Vector3<Robot> = vector![0.0, -0.077, 0.1845];
    pub const RIGHT_INNER_SHOULDER_TO_RIGHT_OUTER_SHOULDER: Vector3<RightInnerShoulder> =
        vector![0.0025, -0.068, -0.0135];
    pub const RIGHT_OUTER_SHOULDER_TO_RIGHT_UPPER_ARM: Vector3<RightOuterShoulder> =
        vector![0.0, -0.044428, 0.0];
    pub const RIGHT_UPPER_ARM_TO_RIGHT_FOREARM: Vector3<RightUpperArm> = vector![0.0, -0.1215, 0.0];

    pub const ROBOT_TO_LEFT_PELVIS: Vector3<Robot> = vector![0.0, 0.096, -0.062];
    pub const LEFT_PELVIS_TO_LEFT_HIP: Vector3<LeftPelvis> = vector![0.0, 0.0, -0.026];
    pub const LEFT_HIP_TO_LEFT_THIGH: Vector3<LeftHip> = vector![0.012, 0.0, -0.0485];
    pub const LEFT_THIGH_TO_LEFT_TIBIA: Vector3<LeftThigh> = vector![-0.014, 0.0, -0.117];
    pub const LEFT_TIBIA_TO_LEFT_ANKLE: Vector3<LeftTibia> = vector![0.00019706, 0.0002, -0.24519];
    pub const LEFT_FOOT_TO_LEFT_SOLE: Vector3<LeftFoot> = vector![0.0, 0.0, -0.026896];

    pub const ROBOT_TO_RIGHT_PELVIS: Vector3<Robot> = vector![0.0, -0.096, -0.062];
    pub const RIGHT_PELVIS_TO_RIGHT_HIP: Vector3<RightPelvis> = vector![0.0, 0.0, -0.026];
    pub const RIGHT_HIP_TO_RIGHT_THIGH: Vector3<RightHip> = vector![0.012, 0.0, -0.0485];
    pub const RIGHT_THIGH_TO_RIGHT_TIBIA: Vector3<RightThigh> = vector![-0.014, 0.0, -0.117];
    pub const RIGHT_TIBIA_TO_RIGHT_ANKLE: Vector3<RightTibia> =
        vector![0.00019706, -0.0002, -0.24519];
    pub const RIGHT_FOOT_TO_RIGHT_SOLE: Vector3<RightFoot> = vector![0.0, 0.0, -0.026896];
}
