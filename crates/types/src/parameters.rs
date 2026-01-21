use std::{
    ops::{Index, Range},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use coordinate_systems::{Camera, Field, Ground, NormalizedPixel, Pixel, Robot};
use linear_algebra::{Point2, Vector2, Vector3};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::{
    joints::head::HeadJoints,
    joints::Joints,
    motion_command::{KickVariant, MotionCommand},
    roles::Role,
    step::Step,
};

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct RemoteControlParameters {
    pub walk: Step,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct WhistleDetectionParameters {
    pub detection_band: Range<f32>,
    pub background_noise_scaling: f32,
    pub whistle_scaling: f32,
    pub number_of_chunks: usize,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct BehaviorParameters {
    pub injected_motion_command: Option<MotionCommand>,
    pub lost_ball: LostBallParameters,
    pub optional_roles: Vec<Role>,
    pub path_planning: PathPlanningParameters,
    pub role_positions: RolePositionsParameters,
    pub walk_and_stand: WalkAndStandParameters,
    pub dribbling: DribblingParameters,
    pub search: SearchParameters,
    pub look_action: LookActionParameters,
    pub intercept_ball: InterceptBallParameters,
    pub maximum_lookaround_duration: Duration,
    pub time_to_reach_delay_when_fallen: Duration,
    pub maximum_standup_attempts: u32,
    pub walk_with_velocity: WalkWithVelocityParameters,
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct LookActionParameters {
    pub angle_threshold: f32,
    pub distance_threshold: f32,
    pub look_forward_position: Point2<Ground>,
    pub position_of_interest_switch_interval: Duration,
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct RolePositionsParameters {
    pub defender_aggressive_ring_radius: f32,
    pub defender_passive_ring_radius: f32,
    pub defender_y_offset: f32,
    pub defender_passive_distance: f32,
    pub defender_passive_hysteresis: f32,
    pub left_midfielder_distance_to_ball: f32,
    pub left_midfielder_maximum_x_in_ready_and_when_ball_is_not_free: f32,
    pub left_midfielder_minimum_x: f32,
    pub right_midfielder_distance_to_ball: f32,
    pub right_midfielder_maximum_x_in_ready_and_when_ball_is_not_free: f32,
    pub right_midfielder_minimum_x: f32,
    pub striker_supporter_distance_to_ball: f32,
    pub striker_supporter_maximum_x_in_ready_and_when_ball_is_not_free: f32,
    pub striker_supporter_minimum_x: f32,
    pub keeper_x_offset: f32,
    pub keeper_passive_distance: f32,
    pub striker_distance_to_non_free_center_circle: f32,
    pub striker_kickoff_position: Point2<Field>,
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct SearchParameters {
    pub position_reached_distance: f32,
    pub rotation_per_step: f32,
    pub stand_secs: f32,
    pub turn_secs: f32,
    pub estimated_ball_speed: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct InWalkKicksParameters {
    pub forward: InWalkKickInfoParameters,
    pub turn: InWalkKickInfoParameters,
    pub side: InWalkKickInfoParameters,
}

impl Index<KickVariant> for InWalkKicksParameters {
    type Output = InWalkKickInfoParameters;

    fn index(&self, variant: KickVariant) -> &Self::Output {
        match variant {
            KickVariant::Forward => &self.forward,
            KickVariant::Turn => &self.turn,
            KickVariant::Side => &self.side,
        }
    }
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct InWalkKickInfoParameters {
    pub position: nalgebra::Point2<f32>,
    pub position_offset: nalgebra::Vector2<f32>,
    pub orientation: f32,
    pub reached_x: Range<f32>,
    pub reached_y: Range<f32>,
    pub reached_turn: Range<f32>,
    pub shot_distance: f32,
    pub enabled: bool,
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct DribblingParameters {
    pub hybrid_align_distance: f32,
    pub distance_to_be_aligned: f32,
    pub angle_to_approach_ball_from_threshold: f32,
    pub ignore_robot_when_near_ball_radius: f32,
    pub distance_to_look_directly_at_the_ball: f32,
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct WalkAndStandParameters {
    pub hysteresis: nalgebra::Vector2<f32>,
    pub target_reached_thresholds: nalgebra::Vector2<f32>,
    pub hybrid_align_distance: f32,
    pub normal_distance_to_be_aligned: f32,
    pub defender_distance_to_be_aligned: f32,
    pub defender_hysteresis: nalgebra::Vector2<f32>,
    pub supporter_hysteresis: nalgebra::Vector2<f32>,
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct LostBallParameters {
    pub offset_to_last_ball_location: Vector2<Field>,
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct InterceptBallParameters {
    pub maximum_ball_distance: f32,
    pub minimum_ball_velocity: f32,
    pub minimum_ball_velocity_towards_robot: f32,
    pub minimum_ball_velocity_towards_own_half: f32,
    pub maximum_intercept_distance: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct PathPlanningParameters {
    pub arc_walking_speed: f32,
    pub ball_obstacle_radius: f32,
    pub field_border_weight: f32,
    pub line_walking_speed: f32,
    pub rotation_penalty_factor: f32,
    pub minimum_robot_radius_at_foot_height: f32,
    pub robot_radius_at_foot_height: f32,
    pub robot_radius_at_hip_height: f32,
    pub half_rotation: Duration,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct GameStateFilterParameters {
    pub game_controller_controller_delay: Duration,
    pub playing_message_delay: Duration,
    pub ready_message_delay: Duration,
    pub kick_off_grace_period: Duration,
    pub tentative_finish_duration: Duration,
    pub distance_to_consider_ball_moved_in_kick_off: f32,
    pub whistle_acceptance_goal_distance: Vector2<Field>,
    pub duration_to_keep_observed_ball: Duration,
    pub duration_to_keep_new_penalties: Duration,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct ImageRegionParameters {
    pub bottom: Point2<NormalizedPixel>,
    pub center: Point2<NormalizedPixel>,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct LookAroundParameters {
    pub look_around_timeout: Duration,
    pub quick_search_timeout: Duration,
    pub middle_positions: HeadJoints<f32>,
    pub left_positions: HeadJoints<f32>,
    pub right_positions: HeadJoints<f32>,
    pub halfway_left_positions: HeadJoints<f32>,
    pub halfway_right_positions: HeadJoints<f32>,
    pub initial_left_positions: HeadJoints<f32>,
    pub initial_right_positions: HeadJoints<f32>,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct HeadMotionParameters {
    pub inner_maximum_pitch: f32,
    pub inner_minimum_pitch: f32,
    pub maximum_velocity: HeadJoints<f32>,
    pub maximum_defender_velocity: HeadJoints<f32>,
    pub outer_maximum_pitch: f32,
    pub outer_minimum_pitch: f32,
    pub outer_yaw: f32,
    pub injected_head_joints: Option<HeadJoints<f32>>,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct HslNetworkParameters {
    pub game_controller_return_message_interval: Duration,
    pub remaining_amount_of_messages_to_stop_sending: u16,
    pub silence_interval_between_messages: Duration,
    pub hsl_striker_message_receive_timeout: Duration,
    pub hsl_striker_message_send_interval: Duration,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum MedianModeParameters {
    #[default]
    Disabled,
    ThreePixels,
    FivePixels,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum EdgeDetectionSourceParameters {
    #[default]
    Luminance,
    GreenChromaticity,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct BallProjectionParameters {
    pub detection_noise: Vector2<Pixel>,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct BallFilterNoise {
    pub process_noise_moving: nalgebra::Vector4<f32>,
    pub process_noise_resting: nalgebra::Vector2<f32>,
    pub initial_covariance: nalgebra::Vector4<f32>,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct BallFilterParameters {
    pub hypothesis_timeout: Duration,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct ObstacleFilterParameters {
    pub hypothesis_timeout: Duration,
    pub network_robot_measurement_matching_distance: f32,
    pub sonar_goal_post_matching_distance: f32,
    pub feet_detection_measurement_matching_distance: f32,
    pub goal_post_measurement_matching_distance: f32,
    pub hypothesis_merge_distance: f32,
    pub process_noise: nalgebra::Vector2<f32>,
    pub feet_measurement_noise: nalgebra::Vector2<f32>,
    pub robot_measurement_noise: nalgebra::Vector2<f32>,
    pub sonar_measurement_noise: nalgebra::Vector2<f32>,
    pub network_robot_measurement_noise: nalgebra::Vector2<f32>,
    pub initial_covariance: nalgebra::Vector2<f32>,
    pub measurement_count_threshold: usize,
    pub use_feet_detection_measurements: bool,
    pub use_sonar_measurements: bool,
    pub use_foot_bumper_measurements: bool,
    pub robot_obstacle_radius_at_hip_height: f32,
    pub robot_obstacle_radius_at_foot_height: f32,
    pub unknown_obstacle_radius: f32,
    pub goal_post_obstacle_radius: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct CameraMatrixParameters {
    pub camera_to_head_pitch: f32,
    pub correction_in_robot: Vector3<Robot>,
    pub correction_in_camera: Vector3<Camera>,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct SearchSuggestorParameters {
    pub cells_per_meter: f32,
    pub heatmap_convolution_kernel_weight: f32,
    pub minimum_validity: f32,
    pub own_ball_weight: f32,
    pub team_ball_weight: f32,
    pub rule_ball_weight: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]

pub struct KeeperMotionParameters {
    pub action_radius_center: f32,
    pub minimum_velocity: f32,
    pub action_radius_left: f32,
    pub maximum_ball_distance: f32,
    pub minimum_ball_velocity: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct PenaltyShotDirectionParameters {
    pub moving_distance_threshold: f32,
    pub minimum_velocity: f32,
    pub center_jump_trigger_radius: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct StepPlanningCostFactors {
    pub path_progress: f32,
    pub path_distance: f32,
    pub target_orientation: f32,
    pub walk_orientation: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct StepPlanningOptimizationParameters {
    pub optimizer_steps: usize,
    pub cost_factors: StepPlanningCostFactors,
    pub path_alignment_tolerance: f32,
    pub path_progress_smoothness: f32,
    pub target_orientation_ahead_tolerance: f32,
    pub target_orientation_side_alignment_tolerance: f32,
    pub hybrid_align_distance: f32,
    pub warm_start: bool,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub enum StepPlannerMode {
    #[default]
    Mpc,
    Greedy,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct RLWalkingParameters {
    pub gait_frequency: f32,
    pub stabilizing_interval_compression_factor: f32,
    pub stabilizing_interval_completion_threshold: f32,
    pub number_of_actions: usize,
    pub number_of_observations: usize,
    pub torque_limits: Joints,
    pub normalization: NormalizationParameters,
    pub control: ControlParameters,
    pub walk_command: [f32; 3],
    pub joint_position_smoothing_factor: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct WalkWithVelocityParameters {
    pub max_velocity: f32,
    pub max_angular_velocity: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct NormalizationParameters {
    pub gravity: f32,
    pub linear_velocity: f32,
    pub angular_velocity: f32,
    pub joint_position: f32,
    pub joint_velocity: f32,
    pub clip_actions: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct ControlParameters {
    pub dt: f32,
    pub action_scale: f32,
    pub decimation: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct MotorCommandParameters {
    pub weight: f32,
    pub default_positions: Joints,
    pub proportional_coefficients: Joints,
    pub derivative_coefficients: Joints,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct ObjectDetectionParameters {
    pub enable: bool,
    pub maximum_intersection_over_union: f32,
    pub confidence_threshold: f32,
}
