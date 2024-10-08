use crate::{
    kick_state::KickState, step_plan::StepPlan, step_state::StepState, stiffness::Stiffness as _,
    Context,
};

use super::{kicking::Kicking, standing::Standing, Mode, WalkTransition, Walking};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{
    joints::body::BodyJoints, motion_command::KickVariant, motor_commands::MotorCommands,
    step::Step, support_foot::Side,
};

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Starting {
    pub step: StepState,
}

impl Starting {
    pub fn new(context: &Context, support_side: Side) -> Self {
        let plan = StepPlan::new_from_request(context, Step::ZERO, support_side);
        let step = StepState::new(plan);
        Self { step }
    }
}

impl WalkTransition for Starting {
    fn stand(self, context: &Context) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context)
            || current_step.is_timeouted(context.parameters)
        {
            return Mode::Standing(Standing {});
        }

        Mode::Starting(self)
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

        Mode::Starting(self)
    }

    fn kick(self, context: &Context, variant: KickVariant, kick_side: Side, strength: f32) -> Mode {
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
            if next_support_side == kick_side {
                return Mode::Walking(Walking::new(
                    context,
                    Step::ZERO,
                    current_step.plan.support_side.opposite(),
                    Step::ZERO,
                ));
            }

            let kick = KickState::new(variant, kick_side, strength);
            return Mode::Kicking(Kicking::new(
                context,
                kick,
                current_step.plan.support_side.opposite(),
            ));
        }
        Mode::Starting(self)
    }
}

impl Starting {
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
