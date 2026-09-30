use std::{f32::consts::FRAC_PI_2, time::Duration};

use kinematics::joints::head::HeadJoints;
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::parameters::ImageRegionParameters;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub(crate) struct Parameters {
    pub(crate) joint_control: JointControlParameters,

    /// Desired travel speeds in rad/s for direct position and look-at requests.
    pub(crate) direct_travel_speed: HeadJoints<f32>,
    /// Maximum age of the latest head measurement when answering a request.
    pub(crate) maximum_observation_age: Duration,
    /// Maximum age of the ground-to-robot pose used for look-at requests.
    pub(crate) maximum_ground_pose_age: Duration,

    /// Explicit debug override of behavior requests, still subject to joint control.
    pub(crate) injected_head_joints: Option<HeadJoints<f32>>,

    pub(crate) image_region_parameters: ImageRegionParameters,

    pub(crate) glance: GlanceParameters,
    pub(crate) look_around: ScanParameters,
    pub(crate) search_for_lost_ball: ScanParameters,
}

impl Parameters {
    pub(crate) fn validate(&self) -> Result<(), String> {
        self.joint_control.validate()?;
        if !self
            .direct_travel_speed
            .into_iter()
            .all(|speed| speed.is_finite() && speed > 0.0)
        {
            return Err("direct_travel_speed must contain finite positive speeds".into());
        }
        if self.maximum_observation_age.is_zero() {
            return Err("maximum_observation_age must be positive".into());
        }
        if self.maximum_ground_pose_age.is_zero() {
            return Err("maximum_ground_pose_age must be positive".into());
        }
        if self
            .injected_head_joints
            .is_some_and(|position| !position.into_iter().all(f32::is_finite))
        {
            return Err("injected_head_joints must contain finite positions".into());
        }
        for (name, position) in [
            ("center", self.image_region_parameters.center),
            ("bottom", self.image_region_parameters.bottom),
            ("top", self.image_region_parameters.top),
        ] {
            if ![position.x(), position.y()]
                .into_iter()
                .all(|value| (0.0..=1.0).contains(&value))
            {
                return Err(format!(
                    "image_region_parameters.{name} must contain finite coordinates in [0, 1]"
                ));
            }
        }
        self.look_around
            .validate()
            .map_err(|error| format!("look_around.{error}"))?;
        self.search_for_lost_ball
            .validate()
            .map_err(|error| format!("search_for_lost_ball.{error}"))?;
        self.glance
            .validate()
            .map_err(|error| format!("glance.{error}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub(crate) struct GlanceParameters {
    /// Bearing offset on each side, in radians, strictly between zero and pi/2.
    pub(crate) angle: f32,
    /// Positive joint travel speeds in rad/s.
    pub(crate) travel_speed: HeadJoints<f32>,
    /// Time spent looking to each side, including travel.
    pub(crate) phase_duration: Duration,
}

impl GlanceParameters {
    fn validate(&self) -> Result<(), String> {
        if !self.angle.is_finite() || self.angle <= 0.0 || self.angle >= FRAC_PI_2 {
            return Err("angle must be finite and strictly between zero and pi/2".into());
        }
        if !self
            .travel_speed
            .into_iter()
            .all(|speed| speed.is_finite() && speed > 0.0)
        {
            return Err("travel_speed must contain finite positive speeds".into());
        }
        if self.phase_duration.is_zero() {
            return Err("phase_duration must be positive".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScanParameters {
    pub(crate) center: HeadJoints<f32>,
    pub(crate) left: HeadJoints<f32>,
    pub(crate) right: HeadJoints<f32>,
    /// Positive joint travel speeds in rad/s.
    pub(crate) travel_speed: HeadJoints<f32>,
    /// Time spent commanding each waypoint, including travel.
    pub(crate) waypoint_duration: Duration,
}

impl ScanParameters {
    fn validate(&self) -> Result<(), String> {
        for (name, position) in [
            ("center", self.center),
            ("left", self.left),
            ("right", self.right),
        ] {
            if !position.into_iter().all(f32::is_finite) {
                return Err(format!("{name} must contain finite joint positions"));
            }
        }
        if !self
            .travel_speed
            .into_iter()
            .all(|speed| speed.is_finite() && speed > 0.0)
        {
            return Err("travel_speed must contain finite positive speeds".into());
        }
        if self.waypoint_duration.is_zero() {
            return Err("waypoint_duration must be positive".into());
        }
        Ok(())
    }
}

/// Gains and speed limits for commanded positions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub(crate) struct JointControlParameters {
    pub(crate) kp: HeadJoints<f32>,
    pub(crate) kd: HeadJoints<f32>,
    /// Maximum commanded angular velocity in rad/s.
    pub(crate) maximum_velocity: HeadJoints<f32>,
    pub(crate) damping_kd: HeadJoints<f32>,
    pub(crate) reseed_after: Duration,
    pub(crate) warning_interval: Duration,
}

impl JointControlParameters {
    fn validate(&self) -> Result<(), String> {
        if !self
            .maximum_velocity
            .into_iter()
            .all(|v| v.is_finite() && v > 0.0)
        {
            return Err("joint_control.maximum_velocity must be finite and positive".into());
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
        if self.reseed_after.is_zero() || self.warning_interval.is_zero() {
            return Err("joint_control time intervals must be positive".into());
        }
        Ok(())
    }
}
