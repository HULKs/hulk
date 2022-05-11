use macros::SerializeHierarchy;
use nalgebra::{Isometry2, Isometry3, Point3, UnitComplex};

use crate::types::{
    BallPosition, BodyMotionSafeExits, Buttons, CameraMatrices, DispatchingBodyPositions,
    DispatchingHeadPositions, FallProtection, FallState, FilteredGameState, FilteredWhistle,
    GameControllerState, HeadJoints, HeadMotionSafeExits, Joints, Leds, MessageReceivers,
    MotionCommand, MotionSelection, PlannedPath, PrimaryState, ProjectedFieldLines,
    RobotKinematics, SensorData, SitDownPositions, SolePressure, StandUpBackPositions,
    StandUpFrontPositions, StepPlan, SupportFoot, WalkCommand, WalkPositions, WorldState,
};

use super::modules::{PoseEstimation, WalkingEngine};

#[derive(Clone, Debug, Default, SerializeHierarchy)]
pub struct MainOutputs {
    pub fall_protection: Option<FallProtection>,
    pub ball_position: Option<BallPosition>,
    pub buttons: Option<Buttons>,
    pub camera_matrices: Option<CameraMatrices>,
    pub center_of_mass: Option<Point3<f32>>,
    pub current_odometry_to_last_odometry: Option<Isometry2<f32>>,
    pub dispatching_head_positions: Option<DispatchingHeadPositions>,
    pub dispatching_body_positions: Option<DispatchingBodyPositions>,
    #[leaf]
    pub fall_state: Option<FallState>,
    pub filtered_whistle: Option<FilteredWhistle>,
    #[leaf]
    pub filtered_game_state: Option<FilteredGameState>,
    #[leaf]
    pub game_controller_state: Option<GameControllerState>,

    pub ground_to_robot: Option<Isometry3<f32>>,
    pub has_ground_contact: Option<bool>,
    pub positions: Option<Joints>,
    #[dont_serialize]
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
    pub sit_down_positions: Option<SitDownPositions>,
    pub sole_pressure: Option<SolePressure>,
    pub stand_up_back_positions: Option<StandUpBackPositions>,
    pub stand_up_front_positions: Option<StandUpFrontPositions>,
    pub step_plan: Option<StepPlan>,
    pub stiffnesses: Option<Joints>,
    pub support_foot: Option<SupportFoot>,
    pub robot_to_ground: Option<Isometry3<f32>>,
    pub walk_command: Option<WalkCommand>,
    pub walk_positions: Option<WalkPositions>,
    pub world_state: Option<WorldState>,
    pub look_at: Option<HeadJoints>,
    pub look_around: Option<HeadJoints>,
    pub leds: Option<Leds>,
    pub zero_angles_head: Option<HeadJoints>,
}

#[derive(Clone, Debug, Default, SerializeHierarchy)]
pub struct AdditionalOutputs {
    pub walking_engine: Option<WalkingEngine>,
    pub pose_estimation: Option<PoseEstimation>,
    pub projected_field_lines: Option<ProjectedFieldLines>,
}

#[derive(Clone, Debug, Default)]
pub struct Database {
    pub main_outputs: MainOutputs,
    pub additional_outputs: AdditionalOutputs,
}

#[derive(Clone, Debug, Default)]
pub struct PersistentState {
    pub body_motion_safe_exits: BodyMotionSafeExits,
    pub head_motion_safe_exits: HeadMotionSafeExits,
}
