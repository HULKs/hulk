use std::time::Duration;

use crate::{
    feet::Feet,
    kick_state::{KickOverride as _, KickState},
    step_plan::StepPlan,
    step_state::StepState,
    stiffness::Stiffness as _,
    Context,
};

use super::{stopping::Stopping, walking::Walking, Mode, WalkTransition};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{
    joints::body::BodyJoints, motion_command::KickVariant, motor_commands::MotorCommands,
    step_plan::Step, support_foot::Side,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct Kicking {
    pub kick: KickState,
    pub step: StepState,
}

impl Kicking {
    pub fn new(context: &Context, kick: KickState, support_side: Side) -> Self {
        let start_feet =
            Feet::from_joints(context.robot_to_walk, &context.current_joints, support_side);

        let kick_step = kick.get_step(context.kick_steps);
        let base_step = kick_step.base_step;
        let request = match kick.side {
            Side::Left => base_step,
            Side::Right => base_step.mirrored(),
        };
        let end_feet = Feet::end_from_request(context.parameters, request, support_side);

        let step = StepState {
            plan: StepPlan {
                step_duration: kick_step.step_duration,
                start_feet,
                end_feet,
                support_side,
                foot_lift_apex: kick_step.foot_lift_apex,
                midpoint: kick_step.midpoint,
            },
            time_since_start: Duration::ZERO,
            gyro_balancing: Default::default(),
            foot_leveling: Default::default(),
        };
        Self { kick, step }
    }
}

impl WalkTransition for Kicking {
    fn stand(self, context: &Context) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context)
            || current_step.is_timeouted(context.parameters)
        {
            return Mode::Stopping(Stopping::new(
                context,
                current_step.plan.support_side.opposite(),
            ));
        }

        Mode::Kicking(self)
    }

    fn walk(self, context: &Context, step: Step) -> Mode {
        let current_step = self.step;
        if current_step.is_timeouted(context.parameters) {
            return Mode::Walking(Walking::new(
                context,
                Step::ZERO,
                current_step.plan.support_side.opposite(),
                Step::ZERO,
            ));
        }

        if current_step.is_support_switched(context) {
            let kick = self.kick.advance_to_next_step();
            if kick.is_finished(context.kick_steps) {
                return Mode::Walking(Walking::new(
                    context,
                    step,
                    current_step.plan.support_side.opposite(),
                    Step::ZERO,
                ));
            }

            return Mode::Kicking(Kicking::new(
                context,
                kick,
                current_step.plan.support_side.opposite(),
            ));
        }

        Mode::Kicking(self)
    }

    fn kick(
        self,
        context: &Context,
        variant: KickVariant,
        kicking_side: Side,
        strength: f32,
    ) -> Mode {
        let current_step = self.step;
        if current_step.is_timeouted(context.parameters) {
            return Mode::Walking(Walking::new(
                context,
                Step::ZERO,
                current_step.plan.support_side.opposite(),
                Step::ZERO,
            ));
        }

        if current_step.is_support_switched(context) {
            let next_support_side = current_step.plan.support_side.opposite();
            let current_kick = self.kick.advance_to_next_step();
            if !current_kick.is_finished(context.kick_steps) {
                return Mode::Kicking(Kicking::new(context, current_kick, next_support_side));
            }

            // TODO: all kicks require a pre-step
            if next_support_side != kicking_side {
                return Mode::Walking(Walking::new(
                    context,
                    Step::ZERO,
                    next_support_side,
                    Step::ZERO,
                ));
            }
            return Mode::Kicking(Kicking::new(
                context,
                KickState::new(variant, kicking_side, strength),
                next_support_side,
            ));
        }

        Mode::Kicking(self)
    }
}

impl Kicking {
    pub fn compute_commands(&self, context: &Context) -> MotorCommands<BodyJoints> {
        self.step
            .compute_joints(context)
            .override_with_kick(context.kick_steps, &self.kick, &self.step)
            .apply_stiffness(
                context.parameters.stiffnesses.leg_stiffness_walk,
                context.parameters.stiffnesses.arm_stiffness,
            )
    }

    pub fn tick(&mut self, context: &Context) {
        self.step.tick(context);
    }
}
