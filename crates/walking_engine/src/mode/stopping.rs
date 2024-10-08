use crate::{
    kick_state::KickState, step_plan::StepPlan, step_state::StepState, stiffness::Stiffness as _,
    Context,
};

use super::{kicking::Kicking, standing::Standing, walking::Walking, Mode, WalkTransition};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{
    joints::body::BodyJoints, motion_command::KickVariant, motor_commands::MotorCommands,
    step::Step, support_foot::Side,
};

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Stopping {
    pub step: StepState,
}

impl Stopping {
    pub fn new(context: &Context, support_side: Side) -> Self {
        let plan = StepPlan::new_from_request(context, Step::ZERO, support_side);
        let step = StepState::new(plan);
        Self { step }
    }
}

impl WalkTransition for Stopping {
    fn stand(self, context: &Context) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context)
            || current_step.is_timeouted(context.parameters)
        {
            return Mode::Standing(Standing {});
        }

        Mode::Stopping(self)
    }

    fn walk(self, context: &Context, requested_step: Step) -> Mode {
        let current_step = self.step;

        if current_step.is_support_switched(context)
            || current_step.is_timeouted(context.parameters)
        {
            return Mode::Walking(Walking::new(
                context,
                requested_step,
                current_step.plan.support_side.opposite(),
                Step::ZERO,
            ));
        }

        Mode::Stopping(self)
    }

    fn kick(self, context: &Context, variant: KickVariant, side: Side, strength: f32) -> Mode {
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
            let support_side = current_step.plan.support_side.opposite();
            if support_side == side {
                return Mode::Walking(Walking::new(
                    context,
                    Step::ZERO,
                    current_step.plan.support_side.opposite(),
                    Step::ZERO,
                ));
            }

            let kick = KickState::new(variant, side, strength);
            return Mode::Kicking(Kicking::new(
                context,
                kick,
                current_step.plan.support_side.opposite(),
            ));
        }

        Mode::Stopping(self)
    }
}

impl Stopping {
    pub fn compute_commands(&self, context: &Context) -> MotorCommands<BodyJoints> {
        self.step.compute_joints(context).apply_stiffness(
            context.parameters.stiffnesses.leg_stiffness_walk,
            context.parameters.stiffnesses.arm_stiffness,
        )
    }

    pub fn tick(&mut self, context: &Context) {
        self.step.tick(context);
    }
}
