use linear_algebra::{point, vector, Isometry3, Point3, Vector3};

use coordinate_systems::{
    Head, LeftAnkle, LeftHip, LeftInnerShoulder, LeftOuterShoulder, LeftPelvis, LeftSole,
    LeftThigh, LeftTibia, LeftUpperArm, Neck, RightAnkle, RightHip, RightInnerShoulder,
    RightOuterShoulder, RightPelvis, RightSole, RightThigh, RightTibia, RightUpperArm, Robot,
};

#[derive(Debug)]
pub struct RobotDimensions {}

impl RobotDimensions {
    pub const ROBOT_TO_TORSO: Vector3<Robot> = vector![0.0, 0.0, 0.0]; //TODO
    pub const ROBOT_TO_NECK: Vector3<Robot> = vector![0.0056, 0.0, 0.2149];
    pub const NECK_TO_HEAD: Vector3<Neck> = vector![0.0, 0.0, 0.033];

    pub const HEAD_TO_CAMERA: Vector3<Head> = vector![0.05868, 0.00002, 0.09849];

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
    pub const LEFT_ANKLE_TO_LEFT_SOLE: Vector3<LeftAnkle> = vector![0.0, 0.0, 0.0]; // TODO

    pub const ROBOT_TO_RIGHT_PELVIS: Vector3<Robot> = vector![0.0, -0.096, -0.062];
    pub const RIGHT_PELVIS_TO_RIGHT_HIP: Vector3<RightPelvis> = vector![0.0, 0.0, -0.026];
    pub const RIGHT_HIP_TO_RIGHT_THIGH: Vector3<RightHip> = vector![0.012, 0.0, -0.0485];
    pub const RIGHT_THIGH_TO_RIGHT_TIBIA: Vector3<RightThigh> = vector![-0.014, 0.0, -0.117];
    pub const RIGHT_TIBIA_TO_RIGHT_ANKLE: Vector3<RightTibia> =
        vector![0.00019706, -0.0002, -0.24519];
    pub const RIGHT_ANKLE_TO_RIGHT_SOLE: Vector3<RightAnkle> = vector![0.0, 0.0, 0.0]; //TODO

    pub const LEFT_SOLE_OUTLINE: [Point3<LeftSole>; 32] = [
        point![-0.05457, -0.015151, 0.0],
        point![-0.050723, -0.021379, 0.0],
        point![-0.04262, -0.030603, 0.0],
        point![-0.037661, -0.033714, 0.0],
        point![-0.03297, -0.034351, 0.0],
        point![0.0577, -0.038771, 0.0],
        point![0.063951, -0.038362, 0.0],
        point![0.073955, -0.03729, 0.0],
        point![0.079702, -0.03532, 0.0],
        point![0.084646, -0.033221, 0.0],
        point![0.087648, -0.031482, 0.0],
        point![0.091805, -0.027692, 0.0],
        point![0.094009, -0.024299, 0.0],
        point![0.096868, -0.018802, 0.0],
        point![0.099419, -0.01015, 0.0],
        point![0.100097, -0.001573, 0.0],
        point![0.098991, 0.008695, 0.0],
        point![0.097014, 0.016504, 0.0],
        point![0.093996, 0.02418, 0.0],
        point![0.090463, 0.02951, 0.0],
        point![0.084545, 0.0361, 0.0],
        point![0.079895, 0.039545, 0.0],
        point![0.074154, 0.042654, 0.0],
        point![0.065678, 0.046145, 0.0],
        point![0.057207, 0.047683, 0.0],
        point![0.049911, 0.048183, 0.0],
        point![-0.031248, 0.051719, 0.0],
        point![-0.03593, 0.049621, 0.0],
        point![-0.040999, 0.045959, 0.0],
        point![-0.045156, 0.042039, 0.0],
        point![-0.04905, 0.037599, 0.0],
        point![-0.054657, 0.029814, 0.0],
    ];
}

pub fn transform_left_sole_outline<Frame>(
    transform: Isometry3<LeftSole, Frame>,
) -> impl Iterator<Item = Point3<Frame>> {
    RobotDimensions::LEFT_SOLE_OUTLINE
        .into_iter()
        .map(move |point| transform * point)
}

pub fn transform_right_sole_outline<Frame>(
    transform: Isometry3<RightSole, Frame>,
) -> impl Iterator<Item = Point3<Frame>> {
    RobotDimensions::LEFT_SOLE_OUTLINE
        .into_iter()
        .map(move |point| transform * point![point.x(), -point.y(), point.z()])
}
