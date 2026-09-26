use ::kinematics::joints::{Joints, JointsName, arm::ArmJoint, leg::LegJoint};
use color_eyre::eyre::{Result, ensure, eyre};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, time::Duration};

pub const HISTORY_LENGTH: usize = 10;
pub const HISTORY_FRAME_SIZE: usize = 32;
pub const JOINT_COUNT: usize = 22;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ros_z::Message)]
pub enum Policy {
    Walk,
    Kick,
    SoftKick,
    SlowGetUp,
    FastGetUp,
}

impl Policy {
    pub const ALL: [Self; 5] = [
        Self::Walk,
        Self::Kick,
        Self::SoftKick,
        Self::SlowGetUp,
        Self::FastGetUp,
    ];

    pub fn file(self, parameters: &Parameters) -> &str {
        &parameters.policies[&self].model_file
    }

    pub fn dimensions(self) -> (usize, usize) {
        match self {
            Self::Walk => (344, 13),
            Self::Kick | Self::SoftKick => (59, 13),
            Self::SlowGetUp => (73, 22),
            Self::FastGetUp => (72, 22),
        }
    }

    pub fn is_locomotion(self) -> bool {
        matches!(self, Self::Walk | Self::Kick | Self::SoftKick)
    }

    pub fn offset(self, parameters: &Parameters) -> Joints<f32> {
        parameters.policies[&self].offset
    }

    pub fn action_limit(self, parameters: &Parameters) -> f32 {
        parameters.policies[&self].action_limit
    }

    pub fn gains(self, parameters: &Parameters) -> (Joints<f32>, Joints<f32>) {
        let policy = &parameters.policies[&self];
        (policy.kp, policy.kd)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub neural_networks_folder: PathBuf,
    pub inference_threads: usize,
    pub policies: HashMap<Policy, PolicyParameters>,
    pub timing: TimingParameters,
    pub observation: ObservationParameters,
    pub locomotion: LocomotionParameters,
    pub kick: KickParameters,
    pub get_up: GetUpParameters,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[serde(deny_unknown_fields)]
pub struct PolicyParameters {
    pub model_file: String,
    pub offset: Joints<f32>,
    pub kp: Joints<f32>,
    pub kd: Joints<f32>,
    pub action_limit: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[serde(deny_unknown_fields)]
pub struct TimingParameters {
    pub policy_period: Duration,
    pub sensor_period: Duration,
    pub maximum_sensor_age: Duration,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[serde(deny_unknown_fields)]
pub struct ObservationParameters {
    pub quaternion_norm_tolerance: f32,
    pub maximum_velocity_sample_gap_frames: f32,
    pub maximum_walking_velocity_change_degrees: f32,
    pub joint_velocity_scale: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[serde(deny_unknown_fields)]
pub struct LocomotionParameters {
    pub forward_velocity_limits: [f32; 2],
    pub lateral_velocity_limit: f32,
    pub angular_velocity_limit: f32,
    pub base_frequency: f32,
    pub initial_frequency_offset: f32,
    pub frequency_offset_limit: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[serde(deny_unknown_fields)]
pub struct KickParameters {
    pub speed_limits: [f32; 2],
    pub soft_speed_limits: [f32; 2],
    pub ball_position_limit: f32,
    pub ball_velocity_limit: f32,
    pub ball_jump_distance: f32,
    pub ball_position_scale: f32,
    pub ball_velocity_scale: f32,
    pub target_speed_scale: f32,
    pub shift_direction_degrees: [f32; 2],
    pub shift_foot_distance: [f32; 2],
    pub shift_ball_distance: [f32; 2],
    pub shift_distance: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[serde(deny_unknown_fields)]
pub struct GetUpParameters {
    pub front_duration_seconds: f32,
    pub back_duration_seconds: f32,
    pub progress_rate: f32,
}

impl Parameters {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.inference_threads > 0,
            "inference_threads must be positive"
        );
        for policy in Policy::ALL {
            let policy_parameters = self
                .policies
                .get(&policy)
                .ok_or_else(|| eyre!("missing parameters for {policy:?}"))?;
            ensure!(
                !policy_parameters.model_file.is_empty(),
                "missing model file for {policy:?}"
            );
            ensure!(
                policy_parameters.offset.into_iter().all(f32::is_finite),
                "invalid offset for {policy:?}"
            );
            ensure!(
                policy_parameters
                    .kp
                    .into_iter()
                    .chain(policy_parameters.kd)
                    .all(|value| value.is_finite() && value >= 0.0),
                "invalid gains for {policy:?}"
            );
            ensure!(
                policy_parameters.action_limit.is_finite() && policy_parameters.action_limit > 0.0,
                "invalid action limit for {policy:?}"
            );
        }
        let timing = &self.timing;
        ensure!(
            [
                timing.policy_period,
                timing.sensor_period,
                timing.maximum_sensor_age
            ]
            .into_iter()
            .all(|value| !value.is_zero() && value.as_secs_f32().is_finite()),
            "invalid inference timing"
        );
        let observation = &self.observation;
        ensure!(
            observation.quaternion_norm_tolerance > 0.0
                && observation.quaternion_norm_tolerance <= 1.0,
            "quaternion norm tolerance must be in (0, 1]"
        );
        let locomotion = &self.locomotion;
        let kick = &self.kick;
        let get_up = &self.get_up;
        ensure!(
            get_up.progress_rate.is_finite() && get_up.progress_rate > 0.0,
            "get-up progress rate must be finite and positive"
        );
        ensure!(
            [
                get_up.front_duration_seconds / get_up.progress_rate,
                get_up.back_duration_seconds / get_up.progress_rate
            ]
            .into_iter()
            .all(|duration| duration.is_finite() && duration > 0.0),
            "invalid effective get-up duration"
        );
        for (name, value) in [
            (
                "observation.quaternion_norm_tolerance",
                observation.quaternion_norm_tolerance,
            ),
            (
                "observation.maximum_walking_velocity_change_degrees",
                observation.maximum_walking_velocity_change_degrees,
            ),
            (
                "observation.joint_velocity_scale",
                observation.joint_velocity_scale,
            ),
            (
                "locomotion.lateral_velocity_limit",
                locomotion.lateral_velocity_limit,
            ),
            (
                "locomotion.angular_velocity_limit",
                locomotion.angular_velocity_limit,
            ),
            ("locomotion.base_frequency", locomotion.base_frequency),
            (
                "locomotion.frequency_offset_limit",
                locomotion.frequency_offset_limit,
            ),
            ("kick.ball_position_limit", kick.ball_position_limit),
            ("kick.ball_velocity_limit", kick.ball_velocity_limit),
            ("kick.ball_jump_distance", kick.ball_jump_distance),
            ("kick.ball_position_scale", kick.ball_position_scale),
            ("kick.ball_velocity_scale", kick.ball_velocity_scale),
            ("kick.target_speed_scale", kick.target_speed_scale),
            ("kick.shift_distance", kick.shift_distance),
            ("get_up.progress_rate", get_up.progress_rate),
        ] {
            ensure!(
                value.is_finite() && value >= 0.0,
                "{name} must be finite and nonnegative, got {value}"
            );
        }
        ensure!(
            [locomotion.initial_frequency_offset,]
                .into_iter()
                .all(f32::is_finite),
            "non-finite locomotion coefficient"
        );
        ensure!(
            [get_up.front_duration_seconds, get_up.back_duration_seconds]
                .into_iter()
                .all(|value| value.is_finite() && value > 0.0),
            "invalid get-up duration"
        );
        ensure!(
            observation.maximum_velocity_sample_gap_frames.is_finite()
                && observation.maximum_velocity_sample_gap_frames >= 1.0,
            "invalid velocity sample gap"
        );
        ensure!(
            kick.speed_limits[0] >= 0.0 && kick.soft_speed_limits[0] >= 0.0,
            "kick speed limits must be nonnegative"
        );
        for (name, [minimum, maximum]) in [
            (
                "locomotion.forward_velocity_limits",
                locomotion.forward_velocity_limits,
            ),
            ("kick.speed_limits", kick.speed_limits),
            ("kick.soft_speed_limits", kick.soft_speed_limits),
        ] {
            ensure!(
                minimum.is_finite() && maximum.is_finite() && minimum <= maximum,
                "{name} must have finite bounds with minimum <= maximum, got [{minimum}, {maximum}]"
            );
        }
        for (name, [minimum, maximum]) in [
            ("kick.shift_direction_degrees", kick.shift_direction_degrees),
            ("kick.shift_foot_distance", kick.shift_foot_distance),
            ("kick.shift_ball_distance", kick.shift_ball_distance),
        ] {
            ensure!(
                minimum.is_finite() && maximum.is_finite() && minimum < maximum,
                "{name} must have finite bounds with minimum < maximum, got [{minimum}, {maximum}]"
            );
        }
        Ok(())
    }
}

pub fn clip_measurement(position: Joints<f32>, joint_limits: Joints<[f32; 2]>) -> Joints<f32> {
    position
        .into_iter()
        .zip(joint_limits)
        .map(|(value, [minimum, maximum])| value.clamp(minimum, maximum))
        .collect()
}

pub const ARMS: [JointsName; 8] = [
    JointsName::LeftArm(ArmJoint::ShoulderPitch),
    JointsName::LeftArm(ArmJoint::ShoulderRoll),
    JointsName::LeftArm(ArmJoint::ShoulderYaw),
    JointsName::LeftArm(ArmJoint::Elbow),
    JointsName::RightArm(ArmJoint::ShoulderPitch),
    JointsName::RightArm(ArmJoint::ShoulderRoll),
    JointsName::RightArm(ArmJoint::ShoulderYaw),
    JointsName::RightArm(ArmJoint::Elbow),
];

pub const LEGS: [JointsName; 12] = [
    JointsName::LeftLeg(LegJoint::HipPitch),
    JointsName::LeftLeg(LegJoint::HipRoll),
    JointsName::LeftLeg(LegJoint::HipYaw),
    JointsName::LeftLeg(LegJoint::Knee),
    JointsName::LeftLeg(LegJoint::AnkleUp),
    JointsName::LeftLeg(LegJoint::AnkleDown),
    JointsName::RightLeg(LegJoint::HipPitch),
    JointsName::RightLeg(LegJoint::HipRoll),
    JointsName::RightLeg(LegJoint::HipYaw),
    JointsName::RightLeg(LegJoint::Knee),
    JointsName::RightLeg(LegJoint::AnkleUp),
    JointsName::RightLeg(LegJoint::AnkleDown),
];
