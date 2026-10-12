use std::time::Duration;

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
    /// Applies to all joints except the head.
    pub maximum_joint_velocity: f32,
    pub confirmation_duration: Duration,
}

/// Entering `StandingUp` from `Fallen` and leaving it again.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct StandingUpParameters {
    /// Must match the pose Prepare drives the joints into.
    pub start_pose: BodyJoints<f32>,
    pub maximum_start_pose_joint_position_deviation: f32,
    pub maximum_start_pose_joint_velocity: f32,
    /// How long the start pose must hold before `Fallen` reports `ready_for_standup`.
    pub confirmation_duration: Duration,
    /// Without confirmed standing, `StandingUp` ends as `Falling` after this.
    pub timeout: Duration,
}

impl Parameters {
    /// Rejects inconsistent combinations. A nonsensical single value only keeps the detector
    /// from confirming a posture, which fails safe into Damping or Prepare.
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
        Ok(())
    }
}
