use crate::motion::walking_engine::{feet::Feet, kicking::KickState, step_state::StepState};

use super::{super::CycleContext, stopping::Stopping, walking::Walking, Mode, WalkTransition};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{motion_command::KickVariant, step_plan::Step, support_foot::Side};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct Kicking {
    pub kick: KickState,
    pub step: StepState,
}

impl Kicking {
    pub fn new(
        context: &CycleContext,
        kick: KickState,
        support_side: Side,
        start_feet: Feet,
    ) -> Self {
        let started_at = context.cycle_time.start_time - context.cycle_time.last_cycle_duration;
        let base_step = kick.kick_step(context).base_step;
        let step = match kick.side {
            Side::Left => base_step,
            Side::Right => base_step.mirrored(),
        };
        let end_feet = Feet::end_from_request(context, step, support_side);
        let step_duration = context.parameters.base.step_duration;
        let kick_step = kick.kick_step(context);
        let max_swing_foot_lift = kick_step.foot_lift_apex;
        let step = StepState {
            started_at,
            step_duration,
            start_feet,
            end_feet,
            support_side,
            max_swing_foot_lift,
            step_adjustment: Default::default(),
            midpoint: kick_step.midpoint,
        };
        Self { kick, step }
    }
}

impl WalkTransition for Kicking {
    fn stand(self, context: &CycleContext) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) {
            let now = context.cycle_time.start_time;
            let next_support_side = current_step.support_side.opposite();
            let kick = self.kick.advance_to_next_step();
            if kick.is_finished(context) {
                return Mode::Stopping(Stopping::new(
                    context,
                    next_support_side,
                    current_step.feet_at(now, context.parameters).swap_sides(),
                ));
            }
            if next_support_side == kick.side {
                Mode::Walking(Walking::new(
                    context,
                    Step::ZERO,
                    next_support_side,
                    current_step.feet_at(now, context.parameters).swap_sides(),
                    Step::ZERO,
                ))
            } else {
                Mode::Kicking(Kicking::new(
                    context,
                    kick,
                    next_support_side,
                    current_step.feet_at(now, context.parameters).swap_sides(),
                ))
            }
        } else {
            Mode::Kicking(Self {
                kick: self.kick,
                step: current_step.advance(context),
            })
        }
    }

    fn walk(self, context: &CycleContext, step: Step) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) {
            let kick = self.kick.advance_to_next_step();
            let now = context.cycle_time.start_time;
            if kick.is_finished(context) {
                return Mode::Walking(Walking::new(
                    context,
                    step,
                    current_step.support_side.opposite(),
                    current_step.feet_at(now, context.parameters).swap_sides(),
                    Step::ZERO,
                ));
            }
            Mode::Kicking(Kicking::new(
                context,
                kick,
                current_step.support_side.opposite(),
                current_step.feet_at(now, context.parameters).swap_sides(),
            ))
        } else {
            Mode::Kicking(Self {
                kick: self.kick,
                step: current_step.advance(context),
            })
        }
    }

    fn kick(self, context: &CycleContext, variant: KickVariant, side: Side, strength: f32) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) {
            let support_side = current_step.support_side.opposite();
            let kick = self.kick.advance_to_next_step();
            let kick = if kick.is_finished(context) {
                KickState::new(variant, side, strength)
            } else {
                kick
            };
            let now = context.cycle_time.start_time;
            if support_side == kick.side {
                Mode::Walking(Walking::new(
                    context,
                    Step::ZERO,
                    current_step.support_side.opposite(),
                    current_step.feet_at(now, context.parameters).swap_sides(),
                    Step::ZERO,
                ))
            } else {
                Mode::Kicking(Kicking::new(
                    context,
                    kick,
                    current_step.support_side.opposite(),
                    current_step.feet_at(now, context.parameters).swap_sides(),
                ))
            }
        } else {
            Mode::Kicking(Self {
                kick: self.kick,
                step: current_step.advance(context),
            })
        }
    }
}
