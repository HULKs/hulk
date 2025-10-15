
use coordinate_systems::{
    Head, LeftAnkle, LeftFoot, LeftForearm, LeftHip, LeftInnerShoulder,
    LeftOuterShoulder, LeftPelvis, LeftSole, LeftThigh, LeftTibia, LeftUpperArm, Neck,
    RightAnkle, RightFoot, RightForearm, RightHip, RightInnerShoulder,
    RightOuterShoulder, RightPelvis, RightSole, RightThigh, RightTibia, RightUpperArm,
    Robot,
};
use linear_algebra::{Isometry3, Orientation3, Vector3};
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
    Isometry3::from_rotation(Vector3::y_axis() * angles.pitch)
}

// TODO: translation
// left arm
pub fn left_inner_shoulder_to_robot(
    angles: &ArmJoints<f32>,
) -> Isometry3<LeftInnerShoulder, Robot> {
    Isometry3::from_parts(
        RobotDimensions::ROBOT_TO_LEFT_SHOULDER,
        Orientation3::new(Vector3::y_axis() * angles.shoulder_pitch),
    )
}

pub fn left_outer_shoulder_to_left_inner_shoulder(
    angles: &ArmJoints<f32>,
) -> Isometry3<LeftOuterShoulder, LeftInnerShoulder> {
    Isometry3::from_rotation(Vector3::x_axis() * angles.shoulder_roll)
}

pub fn left_upper_arm_to_left_outer_shoulder(
    angles: &ArmJoints<f32>,
) -> Isometry3<LeftUpperArm, LeftOuterShoulder> {
    Isometry3::from_rotation(Vector3::y_axis() * angles.shoulder_yaw)
}

pub fn left_forearm_to_left_upper_arm(
    angles: &ArmJoints<f32>,
) -> Isometry3<LeftForearm, LeftUpperArm> {
    Isometry3::from_parts(
        RobotDimensions::LEFT_SHOULDER_TO_LEFT_ELBOW,
        Orientation3::new(Vector3::z_axis() * angles.elbow),
    )
}

// right arm
pub fn right_inner_shoulder_to_robot(
    angles: &ArmJoints<f32>,
) -> Isometry3<RightInnerShoulder, Robot> {
    Isometry3::from_parts(
        RobotDimensions::ROBOT_TO_RIGHT_SHOULDER,
        Orientation3::new(Vector3::y_axis() * angles.shoulder_pitch),
    )
}

pub fn right_outer_shoulder_to_right_inner_shoulder(
    angles: &ArmJoints<f32>,
) -> Isometry3<RightOuterShoulder, RightInnerShoulder> {
    Isometry3::from_rotation(Vector3::x_axis() * angles.shoulder_roll)
}

pub fn right_upper_arm_to_right_outer_shoulder(
    angles: &ArmJoints<f32>,
) -> Isometry3<RightUpperArm, RightOuterShoulder> {
    Isometry3::from_rotation(Vector3::y_axis() * angles.shoulder_yaw)
}

pub fn right_forearm_to_right_upper_arm(
    angles: &ArmJoints<f32>,
) -> Isometry3<RightForearm, RightUpperArm> {
    Isometry3::from_parts(
        RobotDimensions::RIGHT_SHOULDER_TO_RIGHT_ELBOW,
        Orientation3::new(Vector3::z_axis() * angles.elbow),
    )
}
// left leg
pub fn left_pelvis_to_robot(angles: &LegJoints<f32>) -> Isometry3<LeftPelvis, Robot> {
    Isometry3::from_parts(
        RobotDimensions::ROBOT_TO_LEFT_PELVIS,
        Orientation3::new(Vector3::y_axis() * angles.hip_pitch),
    )
}

pub fn left_hip_to_left_pelvis(angles: &LegJoints<f32>) -> Isometry3<LeftHip, LeftPelvis> {
    Isometry3::from_rotation(Vector3::x_axis() * angles.hip_roll)
}

pub fn left_thigh_to_left_hip(angles: &LegJoints<f32>) -> Isometry3<LeftThigh, LeftHip> {
    Isometry3::from_rotation(Vector3::z_axis() * angles.hip_yaw)
}

pub fn left_tibia_to_left_thigh(angles: &LegJoints<f32>) -> Isometry3<LeftTibia, LeftThigh> {
    Isometry3::from_parts(
        RobotDimensions::LEFT_HIP_TO_LEFT_KNEE,
        Orientation3::new(Vector3::y_axis() * angles.knee),
    )
}

pub fn left_ankle_to_left_tibia(angles: &LegJoints<f32>) -> Isometry3<LeftAnkle, LeftTibia> {
    Isometry3::from_parts(
        RobotDimensions::LEFT_KNEE_TO_LEFT_ANKLE,
        Orientation3::new(Vector3::y_axis() * angles.ankle_up),
    )
}

// TODO: wie bewegt sich das Ankle down
pub fn left_foot_to_left_ankle(angles: &LegJoints<f32>) -> Isometry3<LeftFoot, LeftAnkle> {
    Isometry3::from_rotation(Vector3::x_axis() * angles.ankle_up)
}

pub fn left_sole_to_robot(angles: &LegJoints<f32>) -> Isometry3<LeftSole, Robot> {
    left_pelvis_to_robot(angles)
        * left_hip_to_left_pelvis(angles)
        * left_thigh_to_left_hip(angles)
        * left_tibia_to_left_thigh(angles)
        * left_ankle_to_left_tibia(angles)
        * left_foot_to_left_ankle(angles)
        * Isometry3::from(RobotDimensions::LEFT_ANKLE_TO_LEFT_SOLE)
}

// right leg
pub fn right_pelvis_to_robot(angles: &LegJoints<f32>) -> Isometry3<RightPelvis, Robot> {
    Isometry3::from_parts(
        RobotDimensions::ROBOT_TO_RIGHT_PELVIS,
        Orientation3::new(Vector3::y_axis() * angles.hip_pitch),
    )
}

pub fn right_hip_to_right_pelvis(angles: &LegJoints<f32>) -> Isometry3<RightHip, RightPelvis> {
    Isometry3::from_rotation(Vector3::x_axis() * angles.hip_roll)
}

pub fn right_thigh_to_right_hip(angles: &LegJoints<f32>) -> Isometry3<RightThigh, RightHip> {
    Isometry3::from_rotation(Vector3::y_axis() * angles.hip_pitch)
}

pub fn right_tibia_to_right_thigh(angles: &LegJoints<f32>) -> Isometry3<RightTibia, RightThigh> {
    Isometry3::from_parts(
        RobotDimensions::RIGHT_HIP_TO_RIGHT_KNEE,
        Orientation3::new(Vector3::y_axis() * angles.knee),
    )
}

// TODO: wie bewegen sich ankle????
pub fn right_ankle_to_right_tibia(angles: &LegJoints<f32>) -> Isometry3<RightAnkle, RightTibia> {
    Isometry3::from_parts(
        RobotDimensions::RIGHT_KNEE_TO_RIGHT_ANKLE,
        Orientation3::new(Vector3::y_axis() * angles.ankle_up),
    )
}

pub fn right_foot_to_right_ankle(angles: &LegJoints<f32>) -> Isometry3<RightFoot, RightAnkle> {
    Isometry3::from_rotation(Vector3::x_axis() * angles.ankle_down)
}

pub fn right_sole_to_robot(angles: &LegJoints<f32>) -> Isometry3<RightSole, Robot> {
    right_pelvis_to_robot(angles)
        * right_hip_to_right_pelvis(angles)
        * right_thigh_to_right_hip(angles)
        * right_tibia_to_right_thigh(angles)
        * right_ankle_to_right_tibia(angles)
        * right_foot_to_right_ankle(angles)
        * Isometry3::from(RobotDimensions::RIGHT_ANKLE_TO_RIGHT_SOLE)
}
