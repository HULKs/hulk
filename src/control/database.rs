use macros::SerializeHierarchy;
use nalgebra::{Isometry2, Isometry3, Point3, UnitComplex};
use serde::Serialize;

use crate::types::{
    BallPosition, Buttons, CameraMatrices, FallState, FilteredGameState, FilteredWhistle,
    GameControllerState, GroundContact, HeadJoints, HeadJointsCommand, Joints, JointsCommand, Leds,
    MessageReceivers, MotionCommand, MotionSafeExits, MotionSelection, PlannedPath, PrimaryState,
    ProjectedFieldLines, RobotKinematics, SensorData, SitDownJoints, SolePressure, StepPlan,
    SupportFoot, WalkCommand, WalkPositions, WorldState,
};

use super::modules::{motion::walking_engine::WalkingEngine, pose_estimation::PoseEstimation};

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy)]
pub struct MainOutputs {
    pub accumulated_odometry: Option<Isometry2<f32>>,
    pub ball_position: Option<BallPosition>,
    pub buttons: Option<Buttons>,
    pub camera_matrices: Option<CameraMatrices>,
    pub center_of_mass: Option<Point3<f32>>,
    pub current_odometry_to_last_odometry: Option<Isometry2<f32>>,
    pub dispatching_positions: Option<Joints>,
    pub fall_protection_command: Option<JointsCommand>,
    #[leaf]
    pub fall_state: Option<FallState>,
    pub filtered_whistle: Option<FilteredWhistle>,
    #[leaf]
    pub filtered_game_state: Option<FilteredGameState>,
    #[leaf]
    pub game_controller_state: Option<GameControllerState>,
    pub ground_to_robot: Option<Isometry3<f32>>,
    pub ground_contact: Option<GroundContact>,
    pub look_around: Option<HeadJoints>,
    pub look_at: Option<HeadJoints>,
    pub positions: Option<Joints>,
    #[dont_serialize]
    #[serde(skip)]
    pub message_receivers: Option<MessageReceivers>,
    pub motion_command: Option<MotionCommand>,
    pub motion_selection: Option<MotionSelection>,
    pub odometry_offset: Option<Isometry2<f32>>,
    pub planned_path: Option<PlannedPath>,
    #[leaf]
    pub primary_state: Option<PrimaryState>,
    pub robot_kinematics: Option<RobotKinematics>,
    #[leaf]
    pub robot_orientation: Option<UnitComplex<f32>>,
    pub robot_to_field: Option<Isometry2<f32>>,
    pub sensor_data: Option<SensorData>,
    pub sit_down_joints: Option<SitDownJoints>,
    pub sole_pressure: Option<SolePressure>,
    pub stand_up_back_positions: Option<Joints>,
    pub stand_up_front_positions: Option<Joints>,
    pub step_plan: Option<StepPlan>,
    pub stiffnesses: Option<Joints>,
    pub support_foot: Option<SupportFoot>,
    pub robot_to_ground: Option<Isometry3<f32>>,
    #[leaf]
    pub walk_command: Option<WalkCommand>,
    pub walk_positions: Option<WalkPositions>,
    pub world_state: Option<WorldState>,
    pub head_joints_command: Option<HeadJointsCommand>,
    pub leds: Option<Leds>,
}

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy)]
pub struct AdditionalOutputs {
    pub accumulated_odometry: Option<Isometry2<f32>>,
    pub walking_engine: Option<WalkingEngine>,
    pub pose_estimation: Option<PoseEstimation>,
    pub projected_field_lines: Option<ProjectedFieldLines>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Database {
    pub main_outputs: MainOutputs,
    pub additional_outputs: AdditionalOutputs,
}

#[derive(Clone, Debug, Default)]
pub struct PersistentState {
    pub motion_safe_exits: MotionSafeExits,
}
