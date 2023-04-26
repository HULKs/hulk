use std::ops::{Index, Range};
use std::{path::PathBuf, time::Duration};

use nalgebra::{Matrix3, Point2, Point3, Vector2, Vector3, Vector4};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{
    ArmJoints, HeadJoints, InitialPose, KickStep, KickVariant, LegJoints, MotionCommand, Players,
    Role, Step,
};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Audio {
    pub whistle_detection: WhistleDetection,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct WhistleDetection {
    pub detection_band: Range<f32>,
    pub background_noise_scaling: f32,
    pub whistle_scaling: f32,
    pub number_of_chunks: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Localization {
    pub circle_measurement_noise: Vector2<f32>,
    pub gradient_convergence_threshold: f32,
    pub gradient_descent_step_size: f32,
    pub hypothesis_prediction_score_reduction_factor: f32,
    pub hypothesis_retain_factor: f32,
    pub initial_hypothesis_covariance: Matrix3<f32>,
    pub initial_hypothesis_score: f32,
    pub initial_poses: Players<InitialPose>,
    pub line_length_acceptance_factor: f32,
    pub line_measurement_noise: Vector2<f32>,
    pub maximum_amount_of_gradient_descent_iterations: usize,
    pub maximum_amount_of_outer_iterations: usize,
    pub minimum_fit_error: f32,
    pub odometry_noise: Vector3<f32>,
    pub use_line_measurements: bool,
    pub good_matching_threshold: f32,
    pub score_per_good_match: f32,
    pub hypothesis_score_base_increase: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct StepPlanner {
    pub injected_step: Option<Step>,
    pub max_step_size: Step,
    pub max_step_size_backwards: f32,
    pub translation_exponent: f32,
    pub rotation_exponent: f32,
    pub inside_turn_ratio: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SupportFootEstimation {
    pub hysteresis: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct HighDetector {
    pub pressure_threshold: f32,
    pub hysteresis: f32,
    pub timeout: Duration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct RoleAssignment {
    pub forced_role: Option<Role>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]

pub struct Behavior {
    pub injected_motion_command: Option<MotionCommand>,
    pub lost_ball: LostBall,
    pub optional_roles: Vec<Role>,
    pub path_planning: PathPlanning,
    pub role_positions: RolePositions,
    pub walk_and_stand: WalkAndStand,
    pub dribbling: Dribbling,
    pub search: Search,
    pub look_action: LookAction,
    pub initial_lookaround_duration: Duration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct LookAction {
    pub angle_threshold: f32,
    pub distance_threshold: f32,
    pub look_forward_position: Point2<f32>,
    pub position_of_interest_switch_interval: Duration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct RolePositions {
    pub defender_aggressive_ring_radius: f32,
    pub defender_passive_ring_radius: f32,
    pub defender_y_offset: f32,
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
    pub striker_distance_to_non_free_ball: f32,
    pub striker_set_position: Vector2<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Search {
    pub position_reached_distance: f32,
    pub rotation_per_step: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct InWalkKicks {
    pub forward: InWalkKickInfo,
    pub turn: InWalkKickInfo,
    pub side: InWalkKickInfo,
}

impl Index<KickVariant> for InWalkKicks {
    type Output = InWalkKickInfo;

    fn index(&self, variant: KickVariant) -> &Self::Output {
        match variant {
            KickVariant::Forward => &self.forward,
            KickVariant::Turn => &self.turn,
            KickVariant::Side => &self.side,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct InWalkKickInfo {
    pub offset: Vector3<f32>,
    pub shot_angle: f32,
    pub reached_thresholds: Vector3<f32>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Dribbling {
    pub hybrid_align_distance: f32,
    pub distance_to_be_aligned: f32,
    pub angle_to_approach_ball_from_threshold: f32,
    pub ignore_robot_when_near_ball_radius: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct WalkAndStand {
    pub hysteresis: Vector2<f32>,
    pub target_reached_thresholds: Vector2<f32>,
    pub hybrid_align_distance: f32,
    pub distance_to_be_aligned: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct LostBall {
    pub offset_to_last_ball_location: Vector2<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct PathPlanning {
    pub robot_radius_at_foot_height: f32,
    pub robot_radius_at_hip_height: f32,
    pub ball_obstacle_radius: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct GameStateFilter {
    pub game_controller_controller_delay: Duration,
    pub playing_message_delay: Duration,
    pub ready_message_delay: Duration,
    pub kick_off_grace_period: Duration,
    pub distance_to_consider_ball_moved_in_kick_off: f32,
    pub whistle_acceptance_goal_distance: Vector2<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct WalkingEngine {
    pub arm_stiffness: f32,
    pub backward_foot_support_offset: f32,
    pub base_foot_lift: f32,
    pub base_step_duration: Duration,
    pub emergency_foot_lift: f32,
    pub emergency_step: Step,
    pub emergency_step_duration: Duration,
    pub forward_foot_support_offset: f32,
    pub gyro_balance_factor: f32,
    pub gyro_low_pass_factor: f32,
    pub leg_stiffness_stand: f32,
    pub leg_stiffness_walk: f32,
    pub max_forward_acceleration: f32,
    pub max_leg_adjustment_velocity: LegJoints<f32>,
    pub max_number_of_timeouted_steps: usize,
    pub max_number_of_unstable_steps: usize,
    pub max_step_adjustment: f32,
    pub maximal_step_duration: Duration,
    pub minimal_step_duration: Duration,
    pub number_of_stabilizing_steps: usize,
    pub step_duration_increase: Step,
    pub stable_step_deviation: Duration,
    pub starting_step_duration: Duration,
    pub starting_step_foot_lift: f32,
    pub swing_foot_backwards_imu_leveling_factor: f32,
    pub swing_foot_pitch_error_leveling_factor: f32,
    pub swinging_arms: SwingingArms,
    pub tilt_shift_low_pass_factor: f32,
    pub torso_shift_offset: f32,
    pub torso_tilt_offset: f32,
    pub walk_hip_height: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SwingingArms {
    pub debug_pull_back: bool,
    pub default_roll: f32,
    pub roll_factor: f32,
    pub pitch_factor: f32,
    pub pull_back_joints: ArmJoints<f32>,
    pub pull_tight_joints: ArmJoints<f32>,
    pub pulling_back_duration: Duration,
    pub pulling_tight_duration: Duration,
    pub torso_tilt_compensation_factor: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct KickSteps {
    pub forward: Vec<KickStep>,
    pub turn: Vec<KickStep>,
    pub side: Vec<KickStep>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct HeadMotionLimits {
    pub maximum_yaw: f32,
    pub maximum_pitch_at_center: f32,
    pub maximum_pitch_at_shoulder: f32,
    pub shoulder_yaw_position: f32,
    pub ear_shoulder_avoidance_width: f32,
    pub ear_shoulder_avoidance_pitch_penalty: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct DispatchingHeadInterpolator {
    pub maximum_yaw_velocity: f32,
    pub maximum_pitch_velocity: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct LookAt {
    pub glance_angle: f32,
    pub glance_direction_toggle_interval: Duration,
    pub minimum_bottom_focus_pitch: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct HeadMotion {
    pub outer_maximum_pitch: f32,
    pub inner_maximum_pitch: f32,
    pub outer_yaw: f32,
    pub maximum_velocity: HeadJoints<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct LookAround {
    pub look_around_timeout: Duration,
    pub quick_search_timeout: Duration,
    pub middle_positions: HeadJoints<f32>,
    pub left_positions: HeadJoints<f32>,
    pub right_positions: HeadJoints<f32>,
    pub halfway_left_positions: HeadJoints<f32>,
    pub halfway_right_positions: HeadJoints<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct WhistleFilter {
    pub buffer_length: usize,
    pub minimum_detections: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SonarFilter {
    pub low_pass_filter_coefficient: f32,
    pub maximal_reliable_distance: f32,
    pub minimal_reliable_distance: f32,
    pub maximal_detectable_distance: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SonarObstacle {
    pub sensor_angle: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SplNetwork {
    pub game_controller_return_message_interval: Duration,
    pub remaining_amount_of_messages_to_stop_sending: u16,
    pub silence_interval_between_messages: Duration,
    pub spl_striker_message_receive_timeout: Duration,
    pub spl_striker_message_send_interval: Duration,
    pub striker_trusts_team_ball: Duration,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub enum MedianMode {
    #[default]
    Disabled,
    ThreePixels,
    FivePixels,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub enum EdgeDetectionSource {
    #[default]
    Luminance,
    GreenChromaticity,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct ImageSegmenter {
    pub vertical_edge_threshold: i16,
    pub vertical_edge_detection_source: EdgeDetectionSource,
    pub vertical_median_mode: MedianMode,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FieldBorderDetection {
    pub min_points_per_line: usize,
    pub angle_threshold: f32,
    pub first_line_association_distance: f32,
    pub second_line_association_distance: f32,
    pub horizon_margin: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FieldColorDetection {
    pub red_chromaticity_threshold: f32,
    pub blue_chromaticity_threshold: f32,
    pub lower_green_chromaticity_threshold: f32,
    pub upper_green_chromaticity_threshold: f32,
    pub green_luminance_threshold: u8,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct BallDetection {
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
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct ImageReceiver {
    pub resolution: i32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct LineDetection {
    pub allowed_line_length_in_field: Range<f32>,
    pub check_line_distance: bool,
    pub check_line_length: bool,
    pub check_line_segments_projection: bool,
    pub gradient_alignment: f32,
    pub maximum_distance_to_robot: f32,
    pub maximum_fit_distance_in_pixels: f32,
    pub maximum_gap_on_line: f32,
    pub maximum_number_of_lines: usize,
    pub maximum_projected_segment_length: f32,
    pub minimum_number_of_points_on_line: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct BallFilter {
    pub hypothesis_timeout: Duration,
    pub measurement_matching_distance: f32,
    pub hypothesis_merge_distance: f32,
    pub process_noise: Vector4<f32>,
    pub measurement_noise: Vector2<f32>,
    pub initial_covariance: Vector4<f32>,
    pub visible_validity_exponential_decay_factor: f32,
    pub hidden_validity_exponential_decay_factor: f32,
    pub validity_discard_threshold: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct StandUp {
    pub gyro_low_pass_filter_coefficient: f32,
    pub gyro_low_pass_filter_tolerance: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct ButtonFilter {
    pub head_buttons_timeout: Duration,
    pub calibration_buttons_timeout: Duration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct ObstacleFilter {
    pub hypothesis_timeout: Duration,
    pub network_robot_measurement_matching_distance: f32,
    pub sonar_goal_post_matching_distance: f32,
    pub feet_detection_measurement_matching_distance: f32,
    pub robot_detection_measurement_matching_distance: f32,
    pub goal_post_measurement_matching_distance: f32,
    pub hypothesis_merge_distance: f32,
    pub process_noise: Vector2<f32>,
    pub feet_measurement_noise: Vector2<f32>,
    pub robot_measurement_noise: Vector2<f32>,
    pub sonar_measurement_noise: Vector2<f32>,
    pub network_robot_measurement_noise: Vector2<f32>,
    pub initial_covariance: Vector2<f32>,
    pub measurement_count_threshold: usize,
    pub use_feet_detection_measurements: bool,
    pub use_robot_detection_measurements: bool,
    pub use_sonar_measurements: bool,
    pub robot_obstacle_radius_at_hip_height: f32,
    pub robot_obstacle_radius_at_foot_height: f32,
    pub unknown_obstacle_radius: f32,
    pub goal_post_obstacle_radius: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FallStateEstimation {
    pub linear_acceleration_low_pass_factor: f32,
    pub angular_velocity_low_pass_factor: f32,
    pub roll_pitch_low_pass_factor: f32,
    pub gravitational_acceleration_threshold: f32,
    pub falling_angle_threshold: Vector2<f32>,
    pub fallen_timeout: Duration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct PerspectiveGridCandidatesProvider {
    pub minimum_radius: f32,
    pub fallback_radius: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct CameraMatrixParameters {
    pub extrinsic_rotations: Vector3<f32>,
    pub focal_lengths: Vector2<f32>,
    pub cc_optical_center: Point2<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FallProtection {
    pub ground_impact_head_stiffness: f32,
    pub arm_stiffness: f32,
    pub left_arm_positions: ArmJoints<f32>,
    pub right_arm_positions: ArmJoints<f32>,
    pub leg_stiffness: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct ProjectedLimbs {
    pub torso_bounding_polygon: Vec<Point3<f32>>,
    pub lower_arm_bounding_polygon: Vec<Point3<f32>>,
    pub upper_arm_bounding_polygon: Vec<Point3<f32>>,
    pub knee_bounding_polygon: Vec<Point3<f32>>,
    pub foot_bounding_polygon: Vec<Point3<f32>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct RobotDetection {
    pub enable: bool,
    pub amount_of_segments_factor: f32,
    pub amount_score_exponent: f32,
    pub cluster_cone_radius: f32,
    pub cluster_distance_score_range: Range<f32>,
    pub detection_box_width: f32,
    pub ignore_ball_segments: bool,
    pub ignore_line_segments: bool,
    pub luminance_score_exponent: f32,
    pub maximum_cluster_distance: f32,
    pub minimum_cluster_score: f32,
    pub minimum_consecutive_segments: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct PenaltyShotDirectionEstimation {
    pub moving_distance_threshold: f32,
}
