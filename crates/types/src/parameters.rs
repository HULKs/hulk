use std::{ops::Range, path::PathBuf, time::Duration};

use hsl_network_messages::PlayerNumber;
use kinematics::joints::{Joints, head::HeadJoints};
use ros_z::Message;
use serde::{Deserialize, Serialize};

use coordinate_systems::{Camera, Field, Ground, NormalizedPixel, Pixel, Robot};
use linear_algebra::{Framed, Point2, Vector2, Vector3};

use crate::{field_color::FieldColorParameters, motion_command::MotionCommand, players::Players};

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
    pub search: SearchParameters,
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

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct KickOffPose {
    pub position: Point2<Field>,
    pub rotation: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct KickoffParameters {
    pub striker_position: Point2<Field>,
    pub standard_positions: Players<KickOffPose>,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct SearchParameters {
    pub position_reached_distance: f32,
    pub rotation_per_step: f32,
    pub stand_secs: f32,
    pub turn_secs: f32,
    pub estimated_ball_speed: f32,
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
    pub remaining_amount_of_messages_to_stop_sending: u16,
    pub silence_interval_between_messages: Duration,
    pub hsl_striker_message_receive_timeout: Duration,
    pub hsl_state_message_send_interval: Duration,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub enum MedianModeParameters {
    #[default]
    Disabled,
    ThreePixels,
    FivePixels,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub enum EdgeDetectionSourceParameters {
    #[default]
    Luminance,
    GreenChromaticity,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct BallProjectionParameters {
    pub detection_noise: Vector2<Pixel>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
pub struct BallFilterNoise {
    pub detection_noise: Vector2<Pixel>,
    pub process_noise_moving: nalgebra::Vector4<f32>,
    pub process_noise_resting: nalgebra::Vector2<f32>,
    pub initial_covariance: nalgebra::Vector4<f32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
pub struct BallFilterParameters {
    /// Trust fresh localization for field-boundary confidence weighting and decay.
    /// Set false when localization is uncertain; this is an operator assertion,
    /// not an automatically estimated localization-quality signal.
    pub good_localization: bool,
    /// Maximum absolute offset between image exposure and camera geometry.
    pub maximum_camera_matrix_age: Duration,
    /// Additional clearance in metres after the whole ball crosses the playing
    /// field boundary, before either confidence weighting or stored decay starts.
    /// Invalid/negative values act as zero.
    pub field_boundary_margin: f32,
    /// Distance beyond the boundary margin for an e-fold reduction; <= 0 disables.
    pub field_boundary_confidence_decay_distance: f32,
    /// Maximum additional confidence decay per second outside the field.
    /// Zero preserves historical recordings without time-based field decay.
    pub field_boundary_validity_decay_rate: f32,
    /// Maximum projected detection distance in Ground metres; zero disables.
    pub maximum_detection_distance: f32,
    pub hypothesis_timeout: Duration,
    /// Confirmed clear-view miss time before deleting a hypothesis; zero disables.
    pub visible_missed_detection_timeout: Duration,
    /// Continuous clear-view miss time for a nearby ball; zero keeps legacy behavior.
    pub near_visible_missed_detection_timeout: Duration,
    /// Ground distance in metres for the fast near-ball miss rule; zero disables it.
    pub near_visible_missed_detection_distance: f32,
    /// Maximum age of the obstacle model used to establish a clear camera view.
    pub maximum_obstacle_time_difference: Duration,
    pub maximum_number_of_hypotheses: usize,
    pub ball_confidence_threshold: f32,
    pub log_likelihood_of_zero_velocity_threshold: f32,
    /// Optional speed-based moving-to-rest transition, in m/s; zero disables.
    pub resting_velocity_threshold: f32,
    pub hypothesis_merge_distance: f32,
    pub visible_validity_exponential_decay_factor: f32,
    pub hidden_validity_exponential_decay_factor: f32,
    /// Unmatched hidden-track decay per second. None preserves the legacy factor.
    pub hidden_validity_decay_rate: Option<f32>,
    /// Clear-view unmatched-track decay per second. None preserves the legacy factor.
    pub visible_missed_validity_decay_rate: Option<f32>,
    /// Additional decay per second for clearly missed balls in kick range.
    /// None preserves historical behavior; zero applies no extra near-ball decay.
    pub near_visible_missed_validity_decay_rate: Option<f32>,
    /// Extra decay per second for unmatched competitors of a persistently observed,
    /// confident leader. None preserves legacy behavior; zero disables the penalty.
    pub competing_hypothesis_validity_decay_rate: Option<f32>,
    /// Fraction of bounded confidence inherited when spawning near a recent track.
    /// None preserves legacy spawn confidence; zero disables the bonus.
    pub nearby_spawn_validity_factor: Option<f32>,
    pub validity_output_threshold: f32,
    pub validity_discard_threshold: f32,
    pub velocity_decay_factor: f32,
    pub noise: BallFilterNoise,
    pub maximum_matching_cost: f32,
    /// Optional physical association gate in metres. Zero preserves legacy
    /// covariance-only association; positive values reject distant percepts.
    pub maximum_matching_distance: f32,
    /// Physical gate after a gap for quiet balls outside kicking reach; zero disables.
    pub reacquisition_matching_distance: f32,
    /// Maximum ratio between observed and projected ball radii, in either
    /// direction. Values <= 1 disable this optional ground-ball geometry gate.
    pub maximum_detection_radius_ratio: f32,
    /// Apply size consistency only within this Ground distance; zero means everywhere.
    pub radius_consistency_maximum_distance: f32,
    /// Ranking penalty per square metre of position covariance trace. Does not
    /// change output eligibility, stored confidence, or hypothesis retention.
    pub hypothesis_uncertainty_weight: f32,
    /// Bound accumulated support for selection only; zero leaves it unbounded.
    pub selection_confidence_cap: f32,
    /// Blend a separate geometry-filtered position estimate; baseline gates availability.
    pub publication_filter_blend: f32,
    /// Relative pixel noise for the optional position estimator.
    pub publication_detection_noise: f32,
    /// Maximum age of the alternate observation used for correction; zero disables this gate.
    pub publication_maximum_age: Duration,
    /// Maximum alternate ball distance for correction; zero disables this gate.
    pub publication_maximum_distance: f32,
    /// Maximum auxiliary/main position-covariance trace ratio for correction.
    /// A finite positive value enables the gate; zero leaves it disabled.
    #[serde(default)]
    pub publication_maximum_covariance_ratio: f32,
    /// Additional field-boundary uncertainty buffer for the auxiliary history.
    /// The main filter's existing margin remains a lower bound.
    pub publication_field_boundary_margin: f32,
    /// Bounded uncertainty penalty for choosing between feasible associations.
    /// Zero preserves legacy assignment; does not change the matching gate.
    pub association_uncertainty_weight: f32,
    /// Position-standard-deviation margin for clear missed-detection evidence.
    /// Zero preserves center-only visibility. Uncertain visibility pauses misses.
    pub visibility_uncertainty_scale: f32,
    /// Legacy compatibility field; rejected associations no longer penalize track validity.
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
pub struct PenaltyShotDirectionParameters {
    pub moving_distance_threshold: f32,
    pub minimum_velocity: f32,
    pub center_jump_trigger_radius: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
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
    pub switch_policies_threshold: Duration,

    pub hybrid_align_distance: f32,
    pub max_alignment_rate: f32,
    pub deceleration_distance: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct NormalizationParameters {
    pub gravity: f32,
    pub linear_velocity: f32,
    pub angular_velocity: f32,
    pub joint_position: f32,
    pub joint_velocity: f32,
    pub clip_actions: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct ControlParameters {
    pub dt: f32,
    pub action_scale: f32,
    pub decimation: f32,
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

#[derive(Clone, Debug, Default, Deserialize, Serialize, Message)]
pub struct ImageSegmenterParameters {
    pub horizontal_edge_threshold: u8,
    pub horizontal_median_mode: MedianModeParameters,
    pub horizontal_stride: usize,
    pub vertical_edge_threshold: u8,
    pub vertical_median_mode: MedianModeParameters,
    pub vertical_stride: usize,
    pub vertical_stride_in_ground: Framed<Ground, f32>,
    pub field_color_detection: FieldColorParameters,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Message)]
pub struct FieldBorderDetectionParameters {
    pub enable: bool,
    pub angle_threshold: f32,
    pub first_line_association_distance: f32,
    pub min_points_per_line: usize,
    pub second_line_association_distance: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Message)]
pub struct LineDetectionParameters {
    pub use_horizontal_segments: bool,
    pub use_vertical_segments: bool,
    pub allowed_line_length_in_field: Range<f32>,
    pub check_edge_types: bool,
    pub check_edge_gradient: bool,
    pub check_line_distance: bool,
    pub check_line_length: bool,
    pub check_line_segments_projection: bool,
    pub gradient_alignment: f32,
    pub gradient_sobel_stride: u32,
    pub margin_for_point_inclusion: f32,
    pub maximum_distance_to_robot: f32,
    pub maximum_fit_distance_in_ground: f32,
    pub maximum_gap_on_line: f32,
    pub maximum_merge_gap_in_pixels: u16,
    pub maximum_number_of_lines: usize,
    pub allowed_projected_segment_length: Range<f32>,
    pub minimum_number_of_points_on_line: usize,
    pub ransac_iterations: usize,
}
