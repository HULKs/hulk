use std::time::Duration;

use kinematics::joints::head::HeadJoints;
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::parameters::ImageRegionParameters;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub joint_control: JointControlParameters,

    pub center_head_position: HeadJoints<f32>,

    pub maximum_defender_velocity: HeadJoints<f32>,
    pub injected_head_joints: Option<HeadJoints<f32>>,

    pub glance_angle: f32,
    pub image_region_parameters: ImageRegionParameters,
    pub glance_direction_toggle_interval: Duration,

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

/// Motion limits describe the generated reference, not guaranteed physical motion.
/// Initialization and reseeding preserve measured velocity, which may exceed
/// `maximum_velocity`. Lowering limits during tracking preserves reference velocity
/// and acceleration, which may temporarily exceed their new maxima while the planner
/// brakes within the jerk limit. Infeasible position bounds instead trigger recovery
/// with zero velocity and acceleration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct JointControlParameters {
    pub kp: HeadJoints<f32>,
    pub kd: HeadJoints<f32>,
    pub maximum_velocity: HeadJoints<f32>,
    pub maximum_acceleration: HeadJoints<f32>,
    pub maximum_jerk: HeadJoints<f32>,
    pub damping_kd: HeadJoints<f32>,
    pub position_tolerance: HeadJoints<f32>,
    pub velocity_tolerance: HeadJoints<f32>,
    /// Wider exit thresholds prevent arrival chatter. Dwell remains a pattern concern.
    pub arrival_exit_factor: f32,
    pub reseed_after: Duration,
    pub warning_interval: Duration,
}

impl JointControlParameters {
    pub fn validate(&self) -> Result<(), String> {
        for (name, values) in [
            ("maximum_velocity", self.maximum_velocity),
            ("maximum_acceleration", self.maximum_acceleration),
            ("maximum_jerk", self.maximum_jerk),
            ("position_tolerance", self.position_tolerance),
            ("velocity_tolerance", self.velocity_tolerance),
        ] {
            if !values.into_iter().all(|v| v.is_finite() && v > 0.0) {
                return Err(format!("joint_control.{name} must be finite and positive"));
            }
        }
        for (name, values) in [
            ("kp", self.kp),
            ("kd", self.kd),
            ("damping_kd", self.damping_kd),
        ] {
            if !values.into_iter().all(|v| v.is_finite() && v >= 0.0) {
                return Err(format!(
                    "joint_control.{name} must be finite and nonnegative"
                ));
            }
        }
        if !self.arrival_exit_factor.is_finite() || self.arrival_exit_factor < 1.0 {
            return Err("joint_control.arrival_exit_factor must be finite and >= 1".into());
        }
        if self.reseed_after.is_zero() || self.warning_interval.is_zero() {
            return Err("joint_control time intervals must be positive".into());
        }
        Ok(())
    }
}
