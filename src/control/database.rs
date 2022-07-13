use nalgebra::{Isometry2, Isometry3, Point2, Point3, UnitComplex};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use types::{
    BallPosition, BodyJointsCommand, Buttons, CameraMatrices, Circle, FallState, FilteredGameState,
    FilteredWhistle, GameControllerState, HeadJoints, HeadJointsCommand, Joints, JointsCommand,
    KickDecision, Leds, Line2, LocalizationUpdate, MotionCommand, MotionSafeExits, MotionSelection,
    Obstacle, PathObstacle, PrimaryState, ProjectedFieldLines, ProjectedLimbs, RobotKinematics,
    Role, SensorData, SolePressure, SonarObstacle, SonarValues, Step, SupportFoot, WalkCommand,
    WorldState,
};

use crate::spl_network::MessageReceivers;

use super::{
    filtering::ScoredPoseFilter,
    modules::{
        ball_filter::BallFilterHypothesis, motion::walking_engine::WalkingEngine,
        obstacle_filter::ObstacleFilterHypothesis,
    },
};

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
    pub has_ground_contact: Option<bool>,
    pub look_around: Option<HeadJoints>,
    pub look_at: Option<HeadJoints>,
    pub positions: Option<Joints>,
    #[dont_serialize]
    #[serde(skip)]
    pub message_receivers: Option<MessageReceivers>,
    #[leaf]
    pub motion_command: Option<MotionCommand>,
    pub motion_selection: Option<MotionSelection>,
    pub network_robot_obstacles: Option<Vec<Point2<f32>>>,
    pub obstacles: Option<Vec<Obstacle>>,
    pub odometry_offset: Option<Isometry2<f32>>,
    #[leaf]
    pub primary_state: Option<PrimaryState>,
    pub robot_kinematics: Option<RobotKinematics>,
    #[leaf]
    pub robot_orientation: Option<UnitComplex<f32>>,
    pub robot_to_field: Option<Isometry2<f32>>,
    #[leaf]
    pub role: Option<Role>,
    pub sensor_data: Option<SensorData>,
    pub sit_down_joints_command: Option<JointsCommand>,
    pub sole_pressure: Option<SolePressure>,
    pub sonar_obstacle: Option<SonarObstacle>,
    pub stand_up_back_positions: Option<Joints>,
    pub stand_up_front_positions: Option<Joints>,
    pub step_plan: Option<Step>,
    pub stiffnesses: Option<Joints>,
    pub support_foot: Option<SupportFoot>,
    pub team_ball: Option<BallPosition>,
    pub robot_to_ground: Option<Isometry3<f32>>,
    #[leaf]
    pub walk_command: Option<WalkCommand>,
    pub walk_joints_command: Option<BodyJointsCommand>,
    pub world_state: Option<WorldState>,
    pub head_joints_command: Option<HeadJointsCommand>,
    pub leds: Option<Leds>,
    pub projected_limbs: Option<ProjectedLimbs>,
}

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy)]
pub struct AdditionalOutputs {
    pub accumulated_odometry: Option<Isometry2<f32>>,
    pub ball_filter_hypotheses: Option<Vec<BallFilterHypothesis>>,
    pub obstacle_filter_hypotheses: Option<Vec<ObstacleFilterHypothesis>>,
    pub walking_engine: Option<WalkingEngine>,
    pub step_adjustment: Option<StepAdjustment>,
    pub projected_field_lines: Option<ProjectedFieldLines>,
    pub localization: Localization,
    pub path_obstacles: Option<Vec<PathObstacle>>,
    pub filtered_balls_in_image_top: Option<Vec<Circle>>,
    pub filtered_balls_in_image_bottom: Option<Vec<Circle>>,
    pub sonar_values: Option<SonarValues>,
    pub kick_decisions: Option<Vec<KickDecision>>,
    pub kick_targets: Option<Vec<Point2<f32>>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct StepAdjustment {
    pub adjustment: f32,
    pub limited_adjustment: f32,
    pub torso_tilt_shift: f32,
    pub forward_balance_limit: f32,
    pub backward_balance_limit: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Localization {
    pub pose_hypotheses: Option<Vec<ScoredPoseFilter>>,
    pub correspondence_lines: Option<Vec<Line2>>,
    pub measured_lines_in_field: Option<Vec<Line2>>,
    pub updates: Option<Vec<Vec<LocalizationUpdate>>>,
    pub fit_errors: Option<Vec<Vec<Vec<Vec<f32>>>>>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Database {
    pub main_outputs: MainOutputs,
    pub additional_outputs: AdditionalOutputs,
}

#[derive(Clone, Debug, Default)]
pub struct PersistentState {
    pub motion_safe_exits: MotionSafeExits,
    pub walk_return_offset: Step,
    pub robot_to_field: Isometry2<f32>,
}
