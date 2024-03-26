use crate::motion::walking_engine::{feet::Feet, kicking::KickState, step_state::StepState};

use super::{
    super::CycleContext, kicking::Kicking, standing::Standing, walking::Walking, Mode,
    WalkTransition,
};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{motion_command::KickVariant, step_plan::Step, support_foot::Side};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct Stopping {
    pub step: StepState,
}

impl Stopping {
    pub fn new(context: &CycleContext, support_side: Side, start_feet: Feet) -> Self {
        let step = StepState::new(context, Step::ZERO, support_side, start_feet);
        Self { step }
    }
}

impl WalkTransition for Stopping {
    fn stand(self, context: &CycleContext) -> Mode {
        if self.step.is_finished(context) {
            Mode::Standing(Standing {})
        } else {
            Mode::Stopping(self)
        }
    }

    fn walk(self, context: &CycleContext, requested_step: Step) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) {
            let now = context.cycle_time.start_time;
            Mode::Walking(Walking::new(
                context,
                requested_step,
                current_step.support_side.opposite(),
                current_step.feet_at(now, context.parameters).swap_sides(),
                Step::ZERO,
            ))
        } else {
            Mode::Stopping(self)
        }
    }

    fn kick(self, context: &CycleContext, variant: KickVariant, side: Side, strength: f32) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) || current_step.is_timeouted(context) {
            let support_side = current_step.support_side.opposite();
            let now = context.cycle_time.start_time;
            if support_side == side {
                Mode::Walking(Walking::new(
                    context,
                    Step::ZERO,
                    current_step.support_side.opposite(),
                    current_step.feet_at(now, context.parameters).swap_sides(),
                    Step::ZERO,
                ))
            } else {
                let kick = KickState::new(variant, side, strength);
                Mode::Kicking(Kicking::new(
                    context,
                    kick,
                    current_step.support_side.opposite(),
                    current_step.feet_at(now, context.parameters).swap_sides(),
                ))
            }
        } else {
            Mode::Stopping(self)
        }
    }
}
