use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{
    joints::body::BodyJoints, motion_command::KickVariant, motor_commands::MotorCommands,
    step_plan::Step, support_foot::Side,
};

use self::{
    kicking::Kicking, standing::Standing, starting::Starting, stopping::Stopping, walking::Walking,
};

use super::{
    arms::Arm,
    balancing::{GyroBalancing, LevelFeet},
    feet::Feet,
    kicking::KickOverride,
    step_state::StepState,
    stiffness::Stiffness,
    CycleContext,
};

pub mod kicking;
pub mod standing;
pub mod starting;
pub mod stopping;
pub mod walking;

pub trait WalkTransition {
    fn stand(self, context: &CycleContext) -> Mode;
    fn walk(self, context: &CycleContext, step: Step) -> Mode;
    fn kick(self, context: &CycleContext, variant: KickVariant, side: Side, strength: f32) -> Mode;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub enum Mode {
    Standing(Standing),
    Starting(Starting),
    Walking(Walking),
    Kicking(Kicking),
    Stopping(Stopping),
}

impl WalkTransition for Mode {
    fn stand(self, context: &CycleContext) -> Mode {
        match self {
            Self::Standing(standing) => standing.stand(context),
            Self::Starting(starting) => starting.stand(context),
            Self::Walking(walking) => walking.stand(context),
            Self::Kicking(kicking) => kicking.stand(context),
            Self::Stopping(stopping) => stopping.stand(context),
        }
    }

    fn walk(self, context: &CycleContext, step: Step) -> Mode {
        match self {
            Self::Standing(standing) => standing.walk(context, step),
            Self::Starting(starting) => starting.walk(context, step),
            Self::Walking(walking) => walking.walk(context, step),
            Self::Kicking(kicking) => kicking.walk(context, step),
            Self::Stopping(stopping) => stopping.walk(context, step),
        }
    }

    fn kick(self, context: &CycleContext, variant: KickVariant, side: Side, strength: f32) -> Mode {
        match self {
            Self::Standing(standing) => standing.kick(context, variant, side, strength),
            Self::Starting(starting) => starting.kick(context, variant, side, strength),
            Self::Walking(walking) => walking.kick(context, variant, side, strength),
            Self::Kicking(kicking) => kicking.kick(context, variant, side, strength),
            Self::Stopping(stopping) => stopping.kick(context, variant, side, strength),
        }
    }
}

impl Mode {
    pub fn compute_commands(
        &self,
        context: &CycleContext,
        left_arm: &Arm,
        right_arm: &Arm,
        gyro: Vector3<f32>,
    ) -> MotorCommands<BodyJoints<f32>> {
        let stiffnesses = &context.parameters.stiffnesses;
        match self {
            Mode::Standing(..) => standing_commands(context, left_arm, right_arm)
                .apply_stiffness(stiffnesses.leg_stiffness_stand, stiffnesses.arm_stiffness),
            Mode::Starting(Starting { step })
            | Mode::Walking(Walking { step, .. })
            | Mode::Stopping(Stopping { step }) => {
                walking_commands(context, step, left_arm, right_arm, gyro)
                    .apply_stiffness(stiffnesses.leg_stiffness_walk, stiffnesses.arm_stiffness)
            }
            Mode::Kicking(Kicking { kick, step }) => {
                walking_commands(context, step, left_arm, right_arm, gyro)
                    .override_with_kick(context, kick, step)
                    .apply_stiffness(stiffnesses.leg_stiffness_walk, stiffnesses.arm_stiffness)
            }
        }
    }
}

fn standing_commands(context: &CycleContext, left_arm: &Arm, right_arm: &Arm) -> BodyJoints<f32> {
    let support_side = Side::Left;
    let feet = Feet::end_from_request(context, Step::ZERO, support_side);
    feet.compute_joints(context, support_side, left_arm, right_arm)
}

fn walking_commands(
    context: &CycleContext,
    step: &StepState,
    left_arm: &Arm,
    right_arm: &Arm,
    gyro: Vector3<f32>,
) -> BodyJoints<f32> {
    let now = context.cycle_time.start_time;
    let feet = step.feet_at(now, context.parameters);
    feet.compute_joints(context, step.support_side, left_arm, right_arm)
        .balance_using_gyro(
            step,
            gyro,
            &context.parameters.gyro_balancing.balance_factors,
        )
        .level_feet(context, step)
}
