use crate::motion::walking_engine::{feet::Feet, kicking::KickState, step_state::StepState};

use super::{
    super::CycleContext, kicking::Kicking, standing::Standing, Mode, WalkTransition, Walking,
};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{motion_command::KickVariant, step_plan::Step, support_foot::Side};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct Starting {
    pub step: StepState,
}

impl Starting {
    pub fn new(context: &CycleContext, support_side: Side, start_feet: Feet) -> Self {
        let step = StepState::new(context, Step::ZERO, support_side, start_feet);
        Starting { step }
    }
}

impl WalkTransition for Starting {
    fn stand(self, context: &CycleContext) -> Mode {
        if self.step.is_support_switched(context) {
            Mode::Standing(Standing {})
        } else {
            Mode::Starting(Self {
                step: self.step.advance(context),
            })
        }
    }

    fn walk(self, context: &CycleContext, requested_step: Step) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) || current_step.is_timeouted(context) {
            let now = context.cycle_time.start_time;
            Mode::Walking(Walking::new(
                context,
                requested_step,
                current_step.support_side.opposite(),
                current_step.feet_at(now, context.parameters).swap_sides(),
                Step::ZERO,
            ))
        } else {
            Mode::Starting(Self {
                step: current_step.advance(context),
            })
        }
    }

    fn kick(self, context: &CycleContext, variant: KickVariant, side: Side, strength: f32) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) || current_step.is_timeouted(context) {
            let next_support_side = current_step.support_side.opposite();
            let now = context.cycle_time.start_time;
            if next_support_side == side {
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
            Mode::Starting(Self {
                step: current_step.advance(context),
            })
        }
    }
}
