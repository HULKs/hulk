use nalgebra::{vector, Vector3};

#[derive(Debug)]
pub struct RobotDimensions {}

impl RobotDimensions {
    pub const ROBOT_TO_TORSO: Vector3<f32> = vector![0.0413, 0.0, 0.12842];
    pub const ROBOT_TO_NECK: Vector3<f32> = vector![0.0, 0.0, 0.2115];
    pub const ROBOT_TO_LEFT_PELVIS: Vector3<f32> = vector![0.0, 0.05, 0.0];
    pub const ROBOT_TO_RIGHT_PELVIS: Vector3<f32> = vector![0.0, -0.05, 0.0];
    pub const HIP_TO_KNEE: Vector3<f32> = vector![0.0, 0.0, -0.1];
    pub const KNEE_TO_ANKLE: Vector3<f32> = vector![0.0, 0.0, -0.1029];
    pub const ANKLE_TO_SOLE: Vector3<f32> = vector![0.0, 0.0, -0.04519];
    pub const ROBOT_TO_LEFT_SHOULDER: Vector3<f32> = vector![0.0, 0.098, 0.185];
    pub const ROBOT_TO_RIGHT_SHOULDER: Vector3<f32> = vector![0.0, -0.098, 0.185];
    pub const LEFT_SHOULDER_TO_LEFT_ELBOW: Vector3<f32> = vector![0.105, 0.015, 0.0];
    pub const RIGHT_SHOULDER_TO_RIGHT_ELBOW: Vector3<f32> = vector![0.105, -0.015, 0.0];
    pub const ELBOW_TO_WRIST: Vector3<f32> = vector![0.05595, 0.0, 0.0];
    pub const NECK_TO_TOP_CAMERA: Vector3<f32> = vector![0.05871, 0.0, 0.06364];
    pub const NECK_TO_BOTTOM_CAMERA: Vector3<f32> = vector![0.05071, 0.0, 0.01774];
}
