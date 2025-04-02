use itertools::Itertools;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use splines::Interpolate;
use std::time::Duration;
use types::{
    joints::{body::BodyJoints, leg::LegJoints},
    motion_command::KickVariant,
    support_foot::Side,
};

use crate::kick_steps::{JointOverride, JointOverrides, KickStep, KickSteps};

use super::step_state::StepState;

#[derive(
    Debug, Copy, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct KickState {
    pub variant: KickVariant,
    /// the foot that is kicking the ball
    pub side: Side,
    pub index: usize,
    pub strength: f32,
}

impl KickState {
    pub fn new(variant: KickVariant, side: Side, strength: f32) -> Self {
        KickState {
            variant,
            side,
            index: 0,
            strength,
        }
    }

    pub fn advance_to_next_step(self) -> Self {
        KickState {
            index: self.index + 1,
            ..self
        }
    }

    pub fn is_finished(&self, kick_steps: &KickSteps) -> bool {
        self.index >= kick_steps.num_steps(self.variant)
    }

    pub fn get_step<'cycle>(&self, kick_steps: &'cycle KickSteps) -> &'cycle KickStep {
        kick_steps.get_step_at(self.variant, self.index)
    }
}

pub trait KickOverride {
    fn override_with_kick(self, kick_steps: &KickSteps, kick: &KickState, step: &StepState)
        -> Self;
}

impl KickOverride for BodyJoints {
    fn override_with_kick(
        self,
        kick_steps: &KickSteps,
        kick: &KickState,
        step: &StepState,
    ) -> Self {
        let kick_step = kick_steps.get_step_at(kick.variant, kick.index);

        let support_overrides = compute_kick_overrides(
            &kick_step.support_overrides,
            step.time_since_start,
            kick.strength,
        );
        let swing_overrides = compute_kick_overrides(
            &kick_step.swing_overrides,
            step.time_since_start,
            kick.strength,
        );

        match step.plan.support_side {
            Side::Left => BodyJoints {
                left_leg: self.left_leg + support_overrides,
                right_leg: self.right_leg + swing_overrides,
                ..self
            },
            Side::Right => BodyJoints {
                left_leg: self.left_leg + swing_overrides,
                right_leg: self.right_leg + support_overrides,
                ..self
            },
        }
    }
}

fn compute_kick_overrides(
    joint_overrides: &JointOverrides,
    t: Duration,
    strength: f32,
) -> LegJoints {
    let hip_pitch = joint_overrides
        .hip_pitch
        .as_ref()
        .map(|overrides| strength * compute_override(overrides, t))
        .unwrap_or(0.0);
    let knee_pitch = joint_overrides
        .knee_pitch
        .as_ref()
        .map(|overrides| strength * compute_override(overrides, t))
        .unwrap_or(0.0);
    let ankle_pitch = joint_overrides
        .ankle_pitch
        .as_ref()
        .map(|overrides| strength * compute_override(overrides, t))
        .unwrap_or(0.0);

    LegJoints {
        hip_yaw_pitch: 0.0,
        hip_pitch,
        hip_roll: 0.0,
        knee_pitch,
        ankle_pitch,
        ankle_roll: 0.0,
    }
}

fn compute_override(overrides: &[JointOverride], t: Duration) -> f32 {
    let Some((start, end)) = overrides
        .iter()
        .tuple_windows()
        .find(|(start, end)| (start.timepoint..end.timepoint).contains(&t))
    else {
        return 0.0;
    };

    let phase_duration = end.timepoint - start.timepoint;
    let t_in_phase = t - start.timepoint;
    let linear_time = (t_in_phase.as_secs_f32() / phase_duration.as_secs_f32()).clamp(0.0, 1.0);
    f32::lerp(linear_time, start.value, end.value)
}
