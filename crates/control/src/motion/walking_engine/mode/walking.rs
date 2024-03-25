use crate::motion::walking_engine::{feet::Feet, kicking::KickState, step_state::StepState};

use super::{super::CycleContext, kicking::Kicking, stopping::Stopping, Mode, WalkTransition};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{motion_command::KickVariant, step_plan::Step, support_foot::Side};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct Walking {
    pub step: StepState,
    pub requested_step: Step,
}

impl Walking {
    pub fn new(
        context: &CycleContext,
        requested_step: Step,
        support_side: Side,
        start_feet: Feet,
        last_requested_step: Step,
    ) -> Self {
        let requested_step = Step {
            forward: last_requested_step.forward
                + (requested_step.forward - last_requested_step.forward)
                    .min(context.parameters.max_forward_acceleration),
            left: requested_step.left,
            turn: requested_step.turn,
        };
        let step = StepState::new(context, requested_step, support_side, start_feet);
        Self {
            step,
            requested_step,
        }
    }
}

impl WalkTransition for Walking {
    fn stand(self, context: &CycleContext) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) {
            let now = context.cycle_time.start_time;
            Mode::Stopping(Stopping::new(
                context,
                current_step.support_side.opposite(),
                current_step.feet_at(now, context.parameters).swap_sides(),
            ))
        } else {
            Mode::Walking(Self {
                step: current_step.advance(context),
                ..self
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
                self.requested_step,
            ))
        } else {
            Mode::Walking(Self {
                step: current_step.advance(context),
                ..self
            })
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
                    self.requested_step,
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
            Mode::Walking(Self {
                step: current_step.advance(context),
                ..self
            })
        }
    }
}
