use std::ops::{Index, Range};
use std::{path::PathBuf, time::Duration};

use nalgebra::{Point2, Vector2, Vector3, Vector4};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::joints::Joints;
use crate::{
    joints::{ArmJoints, HeadJoints, LegJoints},
    kick_step::KickStep,
    motion_command::{KickVariant, MotionCommand},
    roles::Role,
    step_plan::Step,
};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct WhistleDetectionParameters {
    pub detection_band: Range<f32>,
    pub background_noise_scaling: f32,
    pub whistle_scaling: f32,
    pub number_of_chunks: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct StepPlannerParameters {
    pub injected_step: Option<Step>,
    pub max_step_size: Step,
    pub max_step_size_backwards: f32,
    pub translation_exponent: f32,
    pub rotation_exponent: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
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
    pub initial_lookaround_duration: Duration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct LookActionParameters {
    pub angle_threshold: f32,
    pub distance_threshold: f32,
    pub look_forward_position: Point2<f32>,
    pub position_of_interest_switch_interval: Duration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct MotorCommandOptimizerParameters {
    pub offset_reset_threshold: f32,
    pub offset_reset_speed: f32,
    pub offset_reset_offset: f32,
    pub optimization_speed: f32,
    pub optimization_current_threshold: f32,
    pub optimization_direction: Joints<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct RolePositionsParameters {
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
    pub striker_distance_to_non_free_center_circle: f32,
    pub striker_set_position: Vector2<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SearchParameters {
    pub position_reached_distance: f32,
    pub rotation_per_step: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct InWalkKicksParameters {
    pub forward: InWalkKickInfoParameters,
    pub turn: InWalkKickInfoParameters,
    pub side: InWalkKickInfoParameters,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FindKickTargetsParameters {
    pub distance_from_corner: f32,
    pub corner_kick_target_distance_to_goal: f32,
    pub emergency_kick_target_angles: Vec<f32>,
    pub max_kick_around_obstacle_angle: f32,
    pub ball_radius_for_kick_target_selection: f32,
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

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct InWalkKickInfoParameters {
    pub offset: Vector2<f32>,
    pub shot_angle: f32,
    pub reached_thresholds: Vector3<f32>,
    pub shot_distance: f32,
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct DribblingParameters {
    pub hybrid_align_distance: f32,
    pub distance_to_be_aligned: f32,
    pub angle_to_approach_ball_from_threshold: f32,
    pub ignore_robot_when_near_ball_radius: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct WalkAndStandParameters {
    pub hysteresis: Vector2<f32>,
    pub target_reached_thresholds: Vector2<f32>,
    pub hybrid_align_distance: f32,
    pub distance_to_be_aligned: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct LostBallParameters {
    pub offset_to_last_ball_location: Vector2<f32>,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct InterceptBallParameters {
    pub maximum_ball_distance: f32,
    pub minimum_ball_velocity: f32,
    pub minimum_ball_velocity_towards_robot: f32,
    pub minimum_ball_velocity_towards_own_half: f32,
    pub maximum_intercept_distance: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct PathPlanningParameters {
    pub arc_walking_speed: f32,
    pub ball_obstacle_radius: f32,
    pub field_border_weight: f32,
    pub line_walking_speed: f32,
    pub rotation_penalty_factor: f32,
    pub minimum_robot_radius_at_foot_height: f32,
    pub robot_radius_at_foot_height: f32,
    pub robot_radius_at_hip_height: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct GameStateFilterParameters {
    pub game_controller_controller_delay: Duration,
    pub playing_message_delay: Duration,
    pub ready_message_delay: Duration,
    pub kick_off_grace_period: Duration,
    pub distance_to_consider_ball_moved_in_kick_off: f32,
    pub whistle_acceptance_goal_distance: Vector2<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct WalkingEngineParameters {
    pub additional_kick_foot_lift: f32,
    pub arm_stiffness: f32,
    pub backward_foot_support_offset: f32,
    pub base_foot_lift: f32,
    pub base_step_duration: Duration,
    pub emergency_foot_lift: f32,
    pub emergency_step: Step,
    pub emergency_step_duration: Duration,
    pub foot_pressure_threshold: f32,
    pub forward_foot_support_offset: f32,
    pub gyro_balance_factors: LegJoints<f32>,
    pub gyro_low_pass_factor: f32,
    pub imu_pitch_low_pass_factor: f32,
    pub inside_turn_ratio: f32,
    pub leg_stiffness_stand: f32,
    pub leg_stiffness_walk: f32,
    pub max_forward_acceleration: f32,
    pub max_leg_adjustment_velocity: LegJoints<f32>,
    pub max_number_of_timeouted_steps: usize,
    pub max_number_of_unstable_steps: usize,
    pub max_step_adjustment: f32,
    pub maximal_step_duration: Duration,
    pub forward_step_midpoint: f32,
    pub left_step_midpoint: f32,
    pub minimal_step_duration: Duration,
    pub number_of_stabilizing_steps: usize,
    pub stabilization_foot_lift_multiplier: f32,
    pub stabilization_foot_lift_offset: f32,
    pub stabilization_hysteresis: f32,
    pub stable_step_deviation: Duration,
    pub starting_step_duration: Duration,
    pub starting_step_foot_lift: f32,
    pub step_duration_increase: Step,
    pub step_foot_lift_increase: Step,
    pub swing_foot_imu_leveling_factor: f32,
    pub swing_foot_pitch_error_leveling_factor: f32,
    pub swinging_arms: SwingingArmsParameters,
    pub tilt_shift_low_pass_factor: f32,
    pub torso_shift_offset: f32,
    pub torso_tilt_base_offset: f32,
    pub torso_tilt_forward_offset: f32,
    pub torso_tilt_left_offset: f32,
    pub walk_hip_height: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SwingingArmsParameters {
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
pub struct KickStepsParameters {
    pub forward: Vec<KickStep>,
    pub turn: Vec<KickStep>,
    pub side: Vec<KickStep>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct LookAtParameters {
    pub glance_angle: f32,
    pub glance_direction_toggle_interval: Duration,
    pub minimum_bottom_focus_pitch: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct LookAroundParameters {
    pub look_around_timeout: Duration,
    pub quick_search_timeout: Duration,
    pub middle_positions: HeadJoints<f32>,
    pub left_positions: HeadJoints<f32>,
    pub right_positions: HeadJoints<f32>,
    pub halfway_left_positions: HeadJoints<f32>,
    pub halfway_right_positions: HeadJoints<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SplNetworkParameters {
    pub game_controller_return_message_interval: Duration,
    pub remaining_amount_of_messages_to_stop_sending: u16,
    pub silence_interval_between_messages: Duration,
    pub spl_striker_message_receive_timeout: Duration,
    pub spl_striker_message_send_interval: Duration,
    pub striker_trusts_team_ball: Duration,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub enum MedianModeParameters {
    #[default]
    Disabled,
    ThreePixels,
    FivePixels,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub enum EdgeDetectionSourceParameters {
    #[default]
    Luminance,
    GreenChromaticity,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
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
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct BallFilterParameters {
    pub hypothesis_timeout: Duration,
    pub measurement_matching_distance: f32,
    pub hypothesis_merge_distance: f32,
    pub process_noise: Vector4<f32>,
    pub measurement_noise_moving: Vector2<f32>,
    pub measurement_noise_resting: Vector2<f32>,
    pub initial_covariance: Vector4<f32>,
    pub visible_validity_exponential_decay_factor: f32,
    pub hidden_validity_exponential_decay_factor: f32,
    pub validity_discard_threshold: f32,
    pub velocity_decay_factor: f32,
    pub resting_ball_velocity_threshold: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct ObstacleFilterParameters {
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
pub struct FallStateEstimationParameters {
    pub linear_acceleration_low_pass_factor: f32,
    pub angular_velocity_low_pass_factor: f32,
    pub roll_pitch_low_pass_factor: f32,
    pub gravitational_acceleration_threshold: f32,
    pub fallen_timeout: Duration,
    pub falling_angle_threshold_left: Vector2<f32>,
    pub falling_angle_threshold_forward: Vector2<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct CameraMatrixParameters {
    pub extrinsic_rotations: Vector3<f32>,
    pub focal_lengths: Vector2<f32>,
    pub cc_optical_center: Point2<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FallProtectionParameters {
    pub ground_impact_angular_threshold: f32,
    pub ground_impact_head_stiffness: f32,
    pub ground_impact_body_stiffness: f32,
    pub time_free_motion_exit: Duration,
    pub time_prolong_ground_impact: Duration,
    pub left_arm_positions: ArmJoints<f32>,
    pub right_arm_positions: ArmJoints<f32>,
    pub arm_stiffness: f32,
    pub leg_stiffness: f32,
}
