use crate::motion::walking_engine::feet::Feet;

use super::{super::CycleContext, Mode, Starting, WalkTransition};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{motion_command::KickVariant, step_plan::Step, support_foot::Side};

#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct Standing {}

impl WalkTransition for Standing {
    fn stand(self, _context: &CycleContext) -> Mode {
        Mode::Standing(Standing {})
    }

    fn walk(self, context: &CycleContext, step: Step) -> Mode {
        let is_requested_step_towards_left = step.left.is_sign_positive();
        let support_side = if is_requested_step_towards_left {
            Side::Left
        } else {
            Side::Right
        };
        Mode::Starting(Starting::new(
            context,
            support_side,
            Feet::end_from_request(context, Step::ZERO, support_side),
        ))
    }

    fn kick(
        self,
        context: &CycleContext,
        _variant: KickVariant,
        side: Side,
        _strength: f32,
    ) -> Mode {
        let support_side = if side == Side::Left {
            Side::Left
        } else {
            Side::Right
        };
        Mode::Starting(Starting::new(
            context,
            support_side,
            Feet::end_from_request(context, Step::ZERO, support_side),
        ))
    }
}
