use std::ops::Range;
use std::{path::PathBuf, time::Duration};

use macros::SerializeHierarchy;
use nalgebra::{Vector2, Vector3};
use serde::{Deserialize, Serialize};

use crate::types::{
    FieldDimensions, HeadJoints, InitialPose, Joints, MotionCommand, Players, Step,
};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Configuration {
    pub field_dimensions: FieldDimensions,
    pub audio: Audio,
    pub control: Control,
    pub player_number: usize,
    pub spl_network: SplNetwork,
    pub vision_top: Vision,
    pub vision_bottom: Vision,
}

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
pub struct Control {
    pub ball_filter: BallFilter,
    pub behavior: Behavior,
    pub center_head_position: HeadJoints,
    pub fall_state_estimation: FallStateEstimation,
    pub game_state_filter: GameStateFilter,
    pub high_detector: HighDetector,
    pub look_at: LookAt,
    pub look_around: LookAround,
    pub orientation_filter: OrientationFilter,
    pub path_planner: PathPlanner,
    pub penalized_pose: Joints,
    pub pose_estimation: PoseEstimation,
    pub ready_pose: Joints,
    pub set_positions: SetPositions,
    pub step_planner: StepPlanner,
    pub walking_engine: WalkingEngine,
    pub whistle_filter: WhistleFilter,
    pub fall_protection_parameters: FallProtectionParameters,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct PoseEstimation {
    pub line_measurement_noise: Vector2<f32>,
    pub odometry_noise: Vector3<f32>,
    pub minimal_line_length: f32,
    pub angle_similarity_threshold: f32,
    pub maximum_association_distance: f32,
    pub use_line_measurements: bool,
    pub maximum_line_distance: f32,
    pub initial_poses: Players<InitialPose>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct PathPlanner {
    pub hybrid_align_distance: f32,
    pub distance_to_be_aligned: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct StepPlanner {
    pub max_step_size: Step,
    pub max_step_size_backwards: f32,
    pub translation_exponent: f32,
    pub rotation_exponent: f32,
    pub inside_turn_ratio: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct HighDetector {
    pub total_pressure_threshold: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Behavior {
    pub injected_motion_command: Option<MotionCommand>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct GameStateFilter {
    pub max_wait_for_ready_message: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct WalkingEngine {
    pub walk_hip_height: f32,
    pub torso_offset: f32,
    pub short_step_duration: f32,
    pub long_step_duration: f32,
    pub shoulder_pitch_factor: f32,
    pub base_foot_lift: f32,
    pub base_step_duration: Duration,
    pub first_step_foot_lift_factor: f32,
    pub balance_factor: f32,
    pub max_forward_acceleration: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct LookAt {
    pub maximum_yaw: f32,
    pub maximum_yaw_velocity: f32,
    pub maximum_pitch_velocity: f32,
    pub maximum_pitch_at_center: f32,
    pub maximum_pitch_at_shoulder: f32,
    pub yaw_threshold_for_pitch_limit: f32,
    pub bottom_focus_pitch_threshold: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct LookAround {
    pub maximum_yaw: f32,
    pub maximum_yaw_velocity: f32,
    pub maximum_pitch_velocity: f32,
    pub maximum_pitch_at_center: f32,
    pub maximum_pitch_at_shoulder: f32,
    pub yaw_threshold_for_pitch_limit: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct WhistleFilter {
    pub buffer_length: usize,
    pub minimum_detections: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SplNetwork {}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Vision {
    pub ball_detection: BallDetection,
    pub image_segmenter: ImageSegmenter,
    pub image_receiver: ImageReceiver,
    pub line_detection: LineDetection,
    pub field_border_detection: FieldBorderDetection,
    pub perspective_grid_candidates_provider: PerspectiveGridCandidatesProvider,
    pub camera_matrix_parameters: CameraMatrixParameters,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct ImageSegmenter {
    pub horizontal_edge_threshold: i16,
    pub vertical_edge_threshold: i16,
    pub use_vertical_median: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FieldBorderDetection {
    pub min_points_per_line: usize,
    pub angle_threshold: f32,
    pub first_line_association_distance: f32,
    pub second_line_association_distance: f32,
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
    pub check_line_segments_projection: bool,
    pub gradient_alignment: f32,
    pub maximum_distance_from_line: f32,
    pub maximum_gap_on_line: f32,
    pub maximum_projected_segment_length: f32,
    pub minimum_number_of_points_on_line: usize,
    pub maximum_number_of_lines: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct OrientationFilter {
    pub acceleration_threshold: f32,
    pub delta_angular_velocity_threshold: f32,
    pub angular_velocity_bias_weight: f32,
    pub acceleration_weight: f32,
    pub falling_threshold: f32,
    pub force_sensitive_resistor_threshold: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct BallFilter {
    pub last_seen_timeout: Duration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FallStateEstimation {
    pub linear_acceleration_upright_threshold: Vector3<f32>,
    pub low_pass_filter_coefficient: f32,
    pub minimum_angle: Vector2<f32>,
    pub maximum_angle: Vector2<f32>,
    pub minimum_angular_velocity: Vector2<f32>,
    pub maximum_angular_velocity: Vector2<f32>,
    pub fallen_timeout: Duration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct PerspectiveGridCandidatesProvider {
    pub minimum_radius: f32,
    pub fallback_radius: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SetPositions {
    pub defender_front_set_position_goal_center_offset: Vector2<f32>,
    pub defender_left_set_position_goal_center_offset: Vector2<f32>,
    pub defender_right_set_position_goal_center_offset: Vector2<f32>,
    pub keeper_set_position_goal_center_offset: Vector2<f32>,
    pub striker_set_position: Vector2<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct CameraMatrixParameters {
    pub extrinsic_rotations: Vector3<f32>,
    pub focal_lengths: Vector2<f32>,
    pub cc_optical_center: Vector2<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FallProtectionParameters {
    pub ground_impact_head_stiffness: f32,
}
