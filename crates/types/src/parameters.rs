use std::{ops::Range, path::PathBuf, time::Duration};

use hsl_network_messages::PlayerNumber;
use kinematics::joints::head::HeadJoints;
use ros_z::Message;
use serde::{Deserialize, Serialize};

use coordinate_systems::{Camera, Field, Ground, NormalizedField, NormalizedPixel, Pixel, Robot};
use linear_algebra::{Point2, Vector2, Vector3};

use crate::motion_command::MotionCommand;

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct WhistleDetectionParameters {
    pub detection_band: Range<f32>,
    pub background_noise_scaling: f32,
    pub whistle_scaling: f32,
    pub number_of_chunks: usize,
    pub audio_sample_rate: u32,
    pub number_audio_channels: usize,
    pub number_audio_samples: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Message)]
pub struct VoronoiParameters {
    pub grid_resolution: f32,
    pub padding: f32,
    pub forward_weight: f32,
    pub ball_weight: f32,
    pub ball_support_distance: f32,
    pub ball_support_sigma: f32,
    pub centroid_anchor_weight: f32,
    pub centroid_anchor_sigma: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Message)]
pub struct BehaviorParameters {
    pub control: BehaviorControlParameters,
    pub ball: BallBehaviorParameters,
    pub walking: WalkingBehaviorParameters,
    pub kicking: KickingParameters,
    pub goalkeeper: GoalkeeperParameters,
    pub substates: SubstatesParameters,
    pub kickoff: KickoffParameters,
    pub voronoi: VoronoiParameters,
    pub network: HslNetworkParameters,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct BehaviorControlParameters {
    pub allow_switch: AllowSwitchParameters,
    pub injected_motion_command: Option<MotionCommand>,
    pub is_simple: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct BallBehaviorParameters {
    pub last_ball_timeout: Duration,
    pub interception: InterceptBallParameters,
    pub closest_to_ball: ClosestToBallParameters,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct ClosestToBallParameters {
    pub enter_duration: Duration,
    pub exit_duration: Duration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct WalkingBehaviorParameters {
    pub path_planning: PathPlanningParameters,
    pub walk_and_stand: WalkAndStandParameters,
    pub speed: WalkSpeedParameters,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct LookActionParameters {
    pub angle_threshold: f32,
    pub distance_threshold: f32,
    pub look_forward_position: Point2<Ground>,
    pub position_of_interest_switch_interval: Duration,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct GoalkeeperParameters {
    pub player_number: PlayerNumber,
    pub x_offset: f32,
    pub passive_distance: f32,
    pub striker_distance: f32,
    pub kick_away_ball_maximum_robot_distance: f32,
    pub active_defense_maximum_robot_distance: f32,
    pub distance_to_goalpost: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct KickoffParameters {
    pub relative_goalkeeper_position: Point2<NormalizedField>,
    pub relative_striker_position: Point2<NormalizedField>,
    /// Own-kickoff support slots, filled in order by descending active player number.
    pub relative_aggressive_positions: [Point2<NormalizedField>; 3],
    /// Opponent-kickoff slots, filled in order by descending active player number.
    pub relative_defensive_positions: [Point2<NormalizedField>; 4],
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct WalkAndStandParameters {
    pub hysteresis: nalgebra::Vector2<f32>,
    pub goalkeeper_hysteresis: nalgebra::Vector2<f32>,
    pub target_reached_thresholds: nalgebra::Vector2<f32>,
    pub orientation_tolerance: f32,
    pub normal_distance_to_be_aligned: f32,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct InterceptBallParameters {
    pub maximum_ball_distance: f32,
    pub minimum_ball_velocity: f32,
    pub minimum_ball_velocity_towards_robot: f32,
    pub minimum_ball_velocity_towards_own_half: f32,
    pub maximum_intercept_distance: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct PathPlanningParameters {
    pub arc_walking_speed: f32,
    pub ball_obstacle_radius: f32,
    pub field_border_weight: f32,
    pub line_walking_speed: f32,
    pub obstacle_escape_spline_segments: u32,
    pub rotation_penalty_factor: f32,
    pub robot_radius: f32,
    pub half_rotation: Duration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
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

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct ImageRegionParameters {
    pub bottom: Point2<NormalizedPixel>,
    pub center: Point2<NormalizedPixel>,
    pub top: Point2<NormalizedPixel>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
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

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct HeadMotionParameters {
    pub maximum_pitch: f32,
    pub minimum_pitch: f32,
    pub maximum_velocity: HeadJoints<f32>,
    pub maximum_defender_velocity: HeadJoints<f32>,
    pub maximum_yaw: f32,
    pub minimum_yaw: f32,
    pub injected_head_joints: Option<HeadJoints<f32>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct HslNetworkParameters {
    pub game_controller_return_message_interval: Duration,
    pub half_duration: Duration,
    pub extra_half_duration: Duration,
    pub remaining_amount_of_messages_to_stop_sending: u16,
    pub silence_interval_between_messages: Duration,
    pub hsl_striker_message_receive_timeout: Duration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Message)]
pub struct BallFilterNoise {
    pub detection_noise: Vector2<Pixel>,
    pub process_noise_moving: nalgebra::Vector4<f32>,
    pub process_noise_resting: nalgebra::Vector2<f32>,
    pub initial_covariance: nalgebra::Vector4<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Message)]
pub struct BallFilterParameters {
    pub hypothesis_timeout: Duration,
    pub maximum_camera_matrix_age: Duration,
    pub maximum_number_of_hypotheses: usize,
    pub ball_confidence_threshold: f32,
    pub log_likelihood_of_zero_velocity_threshold: f32,
    pub hypothesis_merge_distance: f32,
    pub visible_validity_exponential_decay_factor: f32,
    pub hidden_validity_exponential_decay_factor: f32,
    pub validity_output_threshold: f32,
    pub validity_discard_threshold: f32,
    pub velocity_decay_factor: f32,
    pub noise: BallFilterNoise,
    pub maximum_matching_cost: f32,
    pub maximum_matching_cost_validity_penalty_factor: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct ObstacleFilterParameters {
    pub use_detected_objects: bool,
    pub hypothesis_timeout: Duration,
    pub network_robot_measurement_matching_distance: f32,
    pub object_detection_measurement_matching_distance: f32,
    pub hypothesis_merge_distance: f32,
    pub process_noise: nalgebra::Vector2<f32>,
    pub network_robot_measurement_noise: nalgebra::Vector2<f32>,
    pub goal_post_measurement_noise: nalgebra::Vector2<f32>,
    pub robot_measurement_noise: nalgebra::Vector2<f32>,
    pub robot_confidence_threshold: f32,
    pub goal_post_confidence_threshold: f32,
    pub measurement_count_threshold: usize,
    pub robot_obstacle_radius_at_hip_height: f32,
    pub robot_obstacle_radius_at_foot_height: f32,
    pub person_obstacle_radius: f32,
    pub unknown_obstacle_radius: f32,
    pub goal_post_obstacle_radius: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct CameraMatrixParameters {
    pub camera_to_head_pitch: f32,
    pub correction_in_robot: Vector3<Robot>,
    pub correction_in_camera: Vector3<Camera>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct SearchSuggestorParameters {
    pub cells_per_meter: f32,
    pub heatmap_convolution_kernel_weight: f32,
    pub minimum_validity: f32,
    pub own_ball_weight: f32,
    pub team_ball_weight: f32,
    pub rule_ball_weight: f32,
    pub rule_ball_weight_increment: f32,
    pub tile_switch_hysteresis: f32,
    pub decay_distance_factor: f32,
    pub heatmap_decay_range: Range<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct DetectionParameters {
    pub enable: bool,
    pub neural_networks_folder: PathBuf,
    pub model_name: String,
    pub object_detection_parameters: ObjectDetectionParameters,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct ObjectDetectionParameters {
    pub maximum_intersection_over_union: f32,
    pub minimum_candidate_confidence: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize, ros_z::Message)]
pub struct WalkSpeedParameters {
    pub kicking: f32,
    pub search: f32,
    pub blocking: f32,
    pub minimum_speed: f32,
    pub velocity_fade_distance: f32,
    pub walk_to_kickoff: f32,
}

impl Default for WalkSpeedParameters {
    fn default() -> Self {
        Self {
            kicking: 1.0,
            search: 1.0,
            blocking: 1.0,
            minimum_speed: 0.2,
            velocity_fade_distance: 1.0,
            walk_to_kickoff: 0.5,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct KickingParameters {
    pub allow_schlong: bool,
    pub distance_for_kick: f32,
    pub distance_for_kick_hysteresis: f32,
    pub kick_target_offset_angle: f32,
    pub target_distance_kick_power_threshold: f32,
    pub kick_position_ball_distance: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct BoosterKickingParameters {
    pub kick_message_interval: Duration,
    pub kick_power: KickPowerParameters,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct KickPowerParameters {
    pub rumpelstilzchen: f64,
    pub schlong: f64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct AllowSwitchParameters {
    pub kick: Duration,
    pub prepare: Duration,
    pub stand: Duration,
    pub stand_up: Duration,
    pub walk: Duration,
    pub damping: Duration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Message)]
pub struct SubstatesParameters {
    pub distance_for_kick: f32,
    pub distance_for_kick_hysteresis: f32,
    pub alignment_angle_threshold: f32,
    pub blocking_distance_offset: f32,
    pub corner_kick_blocking_angle: f32,
    pub penalty_kick_target_y_scale: f32,
}
