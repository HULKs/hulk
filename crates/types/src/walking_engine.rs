use std::time::Duration;

use coordinate_systems::{Ground, Robot, Walk};
use linear_algebra::{Isometry3, Point3, Pose3, Vector3};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{
    joints::{arm::ArmJoints, body::BodyJoints, leg::LegJoints},
    step_plan::Step,
    support_foot::Side,
};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct DebugOutput {
    pub center_of_mass_in_ground: Option<Point3<Ground>>,
    pub last_actuated_joints: BodyJoints,
    pub end_support_sole: Option<Pose3<Walk>>,
    pub end_swing_sole: Option<Pose3<Walk>>,
    pub support_side: Side,
    pub robot_to_walk: Isometry3<Robot, Walk>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct WalkingEngineParameters {
    pub base: BaseParameters,
    pub catching_steps: CatchingStepsParameters,
    pub gyro_balancing: GyroBalancingParameters,
    pub max_forward_acceleration: f32,
    pub max_inside_turn: f32,
    pub max_step_duration: Duration,
    pub max_support_foot_lift_speed: f32,
    pub min_step_duration: Duration,
    pub sole_pressure_threshold: f32,
    pub starting_step: StartingStepParameters,
    pub step_midpoint: Step,
    pub stiffnesses: StiffnessesParameters,
    pub swinging_arms: SwingingArmsParameters,
    pub max_level_delta: f32,
    pub max_rotation_speed: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct BaseParameters {
    pub foot_lift_apex: f32,
    pub foot_lift_apex_increase: Step,
    pub foot_offset_left: Vector3<Walk>,
    pub foot_offset_right: Vector3<Walk>,
    pub step_duration: Duration,
    pub step_duration_increase: Step,
    pub step_midpoint: f32,
    pub torso_offset: f32,
    pub torso_tilt: f32,
    pub walk_height: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct StartingStepParameters {
    pub foot_lift_apex: f32,
    pub step_duration: Duration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct StiffnessesParameters {
    pub arm_stiffness: f32,
    pub leg_stiffness_walk: f32,
    pub leg_stiffness_stand: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct GyroBalancingParameters {
    pub balance_factors: LegJoints<f32>,
    pub low_pass_factor: f32,
    pub max_delta: LegJoints<f32>,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct CatchingStepsParameters {
    pub toe_offset: f32,
    pub heel_offset: f32,
    pub max_adjustment: Vector3<Walk>,
    pub midpoint: f32,
    // pub max_tick_delta: Vector2<Walk>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SwingingArmsParameters {
    pub default_roll: f32,
    pub roll_factor: f32,
    pub pitch_factor: f32,
    pub pull_back_joints: ArmJoints<f32>,
    pub pull_tight_joints: ArmJoints<f32>,
    pub pulling_back_duration: Duration,
    pub pulling_tight_duration: Duration,
    pub torso_tilt_compensation_factor: f32,
}
