use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use std::time::Duration;
use types::{
    joints::{body::BodyJoints, leg::LegJoints},
    kick_step::{JointOverride, KickStep},
    motion_command::KickVariant,
    support_foot::Side,
};

use super::{step_state::StepState, CycleContext};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, SerializeHierarchy)]
pub struct KickState {
    variant: KickVariant,
    pub side: Side,
    index: usize,
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

    fn kick_steps<'cycle>(&self, context: &'cycle CycleContext) -> &'cycle [KickStep] {
        match self.variant {
            KickVariant::Forward => &context.kick_steps.forward,
            KickVariant::Turn => &context.kick_steps.turn,
            KickVariant::Side => &context.kick_steps.side,
        }
    }

    pub fn kick_step<'cycle>(&self, context: &'cycle CycleContext) -> &'cycle KickStep {
        &self.kick_steps(context)[self.index]
    }

    pub fn advance_to_next_step(self) -> Self {
        KickState {
            index: self.index + 1,
            ..self
        }
    }

    pub fn is_finished(&self, context: &CycleContext) -> bool {
        self.index >= self.kick_steps(context).len()
    }
}

pub trait KickOverride {
    fn override_with_kick(
        self,
        context: &CycleContext,
        kick: &KickState,
        step: &StepState,
    ) -> BodyJoints<f32>;
}

impl KickOverride for BodyJoints<f32> {
    fn override_with_kick(
        self,
        context: &CycleContext,
        kick: &KickState,
        step: &StepState,
    ) -> Self {
        let kick_step = kick.kick_step(context);
        let overrides = compute_kick_overrides(
            kick_step,
            step.time_since_start(context.cycle_time.start_time),
            kick.strength,
        );
        match step.support_side {
            Side::Left => BodyJoints {
                right_leg: self.right_leg + overrides,
                ..self
            },
            Side::Right => BodyJoints {
                left_leg: self.left_leg + overrides,
                ..self
            },
        }
    }
}

fn compute_kick_overrides(kick_step: &KickStep, t: Duration, strength: f32) -> LegJoints<f32> {
    let hip_pitch = if let Some(overrides) = &kick_step.hip_pitch_overrides {
        strength * compute_override(overrides, t)
    } else {
        0.0
    };
    let ankle_pitch = if let Some(overrides) = &kick_step.ankle_pitch_overrides {
        strength * compute_override(overrides, t)
    } else {
        0.0
    };
    LegJoints {
        hip_yaw_pitch: 0.0,
        hip_pitch,
        hip_roll: 0.0,
        knee_pitch: 0.0,
        ankle_pitch,
        ankle_roll: 0.0,
    }
}

fn compute_override(overrides: &[JointOverride], t: Duration) -> f32 {
    let window = overrides.windows(2).find_map(|window| {
        if t >= window[0].timepoint && t < window[1].timepoint {
            Some((window[0], window[1]))
        } else {
            None
        }
    });

    match window {
        Some((start, end)) => {
            let phase_duration = end.timepoint - start.timepoint;
            let t_in_phase = t - start.timepoint;
            let linear_time =
                (t_in_phase.as_secs_f32() / phase_duration.as_secs_f32()).clamp(0.0, 1.0);
            (1.0 - linear_time) * start.value + linear_time * end.value
        }
        None => 0.0,
    }
}
