use std::ops::{Index, Range};
use std::{path::PathBuf, time::Duration};

use coordinate_systems::{Field, Ground, NormalizedPixel, Pixel};
use linear_algebra::{Point2, Pose2, Vector2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::{
    joints::head::HeadJoints,
    motion_command::{KickVariant, MotionCommand},
    roles::Role,
    step::Step,
};

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
pub struct StepPlannerParameters {
    pub injected_step: Option<Step>,
    pub max_step_size: Step,
    pub max_step_size_backwards: f32,
    pub translation_exponent: f32,
    pub rotation_exponent: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct BehaviorParameters {
    pub injected_motion_command: Option<MotionCommand>,
    pub lost_ball: LostBallParameters,
    pub offense_optional_roles: Vec<Role>,
    pub number_of_defensive_players: usize,
    pub path_planning: PathPlanningParameters,
    pub role_positions: RolePositionsParameters,
    pub walk_and_stand: WalkAndStandParameters,
    pub dribbling: DribblingParameters,
    pub search: SearchParameters,
    pub look_action: LookActionParameters,
    pub intercept_ball: InterceptBallParameters,
    pub maximum_lookaround_duration: Duration,
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
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct RolePositionsParameters {
    pub defender_aggressive_ring_radius: f32,
    pub defender_passive_ring_radius: f32,
    pub defender_y_offset: f32,
    pub defender_passive_distance: f32,
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
    pub striker_kickoff_pose: Pose2<Field>,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct SearchParameters {
    pub position_reached_distance: f32,
    pub rotation_per_step: f32,
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
    pub reached_thresholds: nalgebra::Vector3<f32>,
    pub shot_distance: f32,
    pub enabled: bool,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct DribblingParameters {
    pub hybrid_align_distance: f32,
    pub distance_to_be_aligned: f32,
    pub angle_to_approach_ball_from_threshold: f32,
    pub ignore_robot_when_near_ball_radius: f32,
    pub distance_to_look_directly_at_the_ball: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct WalkAndStandParameters {
    pub hysteresis: nalgebra::Vector2<f32>,
    pub target_reached_thresholds: nalgebra::Vector2<f32>,
    pub hybrid_align_distance: f32,
    pub normal_distance_to_be_aligned: f32,
    pub defender_distance_to_be_aligned: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
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
pub struct SplNetworkParameters {
    pub game_controller_return_message_interval: Duration,
    pub remaining_amount_of_messages_to_stop_sending: u16,
    pub silence_interval_between_messages: Duration,
    pub spl_striker_message_receive_timeout: Duration,
    pub spl_striker_message_send_interval: Duration,
    pub striker_trusts_team_ball: Duration,
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
pub struct BallDetectionParameters {
    pub minimal_radius: f32,
    pub preclassifier_neural_network: PathBuf,
    pub classifier_neural_network: PathBuf,
    pub positioner_neural_network: PathBuf,
    pub maximum_number_of_candidate_evaluations: usize,
    pub preclassifier_confidence_threshold: f32,
    pub classifier_confidence_threshold: f32,
    pub confidence_merge_factor: f32,
    pub correction_proximity_merge_factor: f32,
    pub image_containment_merge_factor: f32,
    pub cluster_merge_radius_factor: f32,
    pub ball_radius_enlargement_factor: f32,
    pub detection_noise: Vector2<Pixel>,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct BallFilterNoise {
    pub process_noise_moving: nalgebra::Vector4<f32>,
    pub process_noise_resting: nalgebra::Vector2<f32>,
    pub measurement_noise: nalgebra::Vector2<f32>,
    pub initial_covariance: nalgebra::Vector4<f32>,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct BallFilterParameters {
    pub hypothesis_timeout: Duration,
    pub measurement_matching_distance: f32,
    pub hypothesis_merge_distance: f32,
    pub visible_validity_exponential_decay_factor: f32,
    pub hidden_validity_exponential_decay_factor: f32,
    pub validity_output_threshold: f32,
    pub validity_discard_threshold: f32,
    pub velocity_decay_factor: f32,
    pub resting_ball_velocity_threshold: f32,
    pub noise: BallFilterNoise,
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
    pub camera_pitch: f32,
    pub extrinsic_rotations: nalgebra::Vector3<f32>,
    pub focal_lengths: nalgebra::Vector2<f32>,
    pub cc_optical_center: nalgebra::Point2<f32>,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct SearchSuggestorParameters {
    pub cells_per_meter: f32,
    pub heatmap_decay_factor: f32,
    pub minimum_validity: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct WideStanceParameters {
    pub maximum_ball_distance: f32,
    pub minimum_ball_velocity: f32,
    pub action_radius: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct PenaltyShotDirectionParameters {
    pub moving_distance_threshold: f32,
    pub minimum_velocity: f32,
    pub center_jump_trigger_radius: f32,
}
