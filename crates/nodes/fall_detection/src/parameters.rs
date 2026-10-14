use std::{f32::consts::FRAC_PI_2, time::Duration};

use kinematics::joints::body::BodyJoints;
use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// Older sensor samples and motion commands are ignored.
    /// A longer gap between sensor samples restarts all confirmations.
    pub maximum_observation_age: Duration,
    /// How long consumers may use a published posture.
    /// A longer gap between sensor samples resets the detector.
    pub detection_lifetime: Duration,
    pub upright: UprightParameters,
    pub falling: FallingParameters,
    pub fallen: FallenParameters,
    pub standing_up: StandingUpParameters,
}

/// Entering `Upright`: standing on extended legs.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct UprightParameters {
    pub maximum_torso_tilt: f32,
    pub maximum_torso_angular_velocity: f32,
    pub maximum_leg_joint_velocity: f32,
    pub minimum_torso_height_above_soles: f32,
    pub confirmation_duration: Duration,
}

/// Leaving `Upright`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct FallingParameters {
    pub minimum_torso_tilt: f32,
    pub confirmation_duration: Duration,
}

/// Entering `Fallen`: lying still.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct FallenParameters {
    pub minimum_torso_tilt: f32,
    pub maximum_torso_angular_velocity: f32,
    /// Limits each non-head joint, even when the total motion is small.
    pub maximum_joint_velocity: f32,
    /// Maximum sum of absolute joint velocities, excluding the head, in rad/s.
    pub maximum_joint_velocity_sum: f32,
    pub confirmation_duration: Duration,
}

/// Entering `StandingUp` from `Fallen` and leaving it again.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct StandingUpParameters {
    /// Must match the pose Prepare drives the joints into.
    pub start_pose: BodyJoints<f32>,
    /// Maximum angle from lying flat on the front or back, in radians.
    pub maximum_start_pose_torso_deviation: f32,
    pub maximum_start_pose_torso_angular_velocity: f32,
    pub maximum_start_pose_joint_position_deviation: f32,
    pub maximum_start_pose_joint_velocity: f32,
    /// Maximum sum of absolute joint velocities, excluding the head, in rad/s.
    pub maximum_start_pose_joint_velocity_sum: f32,
    /// How long torso and joint conditions must hold in `Fallen` before reporting readiness.
    pub confirmation_duration: Duration,
    /// How long the fallen conditions must be interrupted to enable a return to `Fallen`.
    pub movement_confirmation_duration: Duration,
    /// Without confirmed standing, `StandingUp` ends as `Falling` after this.
    pub timeout: Duration,
}

impl Parameters {
    /// Rejects inconsistent combinations and invalid motion limits.
    pub fn validate(&self) -> std::result::Result<(), String> {
        // Also rejects NaN, which would keep a robot Upright forever.
        if !(self.upright.maximum_torso_tilt < self.falling.minimum_torso_tilt
            && self.falling.minimum_torso_tilt < self.fallen.minimum_torso_tilt)
        {
            return Err("torso tilts must increase from upright to falling to fallen".into());
        }
        if self.detection_lifetime <= self.maximum_observation_age {
            return Err("detection_lifetime must exceed maximum_observation_age".into());
        }
        if !(0.0..FRAC_PI_2).contains(&self.standing_up.maximum_start_pose_torso_deviation) {
            return Err("start pose torso deviation must be in [0, pi/2)".into());
        }
        for (name, limit) in [
            (
                "upright.maximum_torso_angular_velocity",
                self.upright.maximum_torso_angular_velocity,
            ),
            (
                "upright.maximum_leg_joint_velocity",
                self.upright.maximum_leg_joint_velocity,
            ),
            (
                "fallen.maximum_torso_angular_velocity",
                self.fallen.maximum_torso_angular_velocity,
            ),
            (
                "fallen.maximum_joint_velocity",
                self.fallen.maximum_joint_velocity,
            ),
            (
                "fallen.maximum_joint_velocity_sum",
                self.fallen.maximum_joint_velocity_sum,
            ),
            (
                "standing_up.maximum_start_pose_torso_angular_velocity",
                self.standing_up.maximum_start_pose_torso_angular_velocity,
            ),
            (
                "standing_up.maximum_start_pose_joint_velocity",
                self.standing_up.maximum_start_pose_joint_velocity,
            ),
            (
                "standing_up.maximum_start_pose_joint_velocity_sum",
                self.standing_up.maximum_start_pose_joint_velocity_sum,
            ),
        ] {
            if !limit.is_finite() || limit <= 0.0 {
                return Err(format!("{name} must be finite and positive"));
            }
        }
        if self.standing_up.maximum_start_pose_torso_angular_velocity
            > self.fallen.maximum_torso_angular_velocity
            || self.standing_up.maximum_start_pose_joint_velocity
                > self.fallen.maximum_joint_velocity
            || self.standing_up.maximum_start_pose_joint_velocity_sum
                > self.fallen.maximum_joint_velocity_sum
        {
            return Err("standup readiness motion limits must not exceed fallen limits".into());
        }
        Ok(())
    }
}
