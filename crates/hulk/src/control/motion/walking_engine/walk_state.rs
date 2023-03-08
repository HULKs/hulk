use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{configuration::KickSteps, KickVariant, Side, Step, WalkCommand};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub enum WalkState {
    Standing,
    Starting(Step),
    Walking(Step),
    Kicking(KickVariant, Side, usize),
    Stopping,
}

impl Default for WalkState {
    fn default() -> Self {
        Self::Standing
    }
}

impl WalkState {
    pub fn next_walk_state(
        self,
        requested_walk_action: WalkCommand,
        swing_side: Side,
        kick_steps: &KickSteps,
    ) -> Self {
        match (self, requested_walk_action) {
            (WalkState::Standing, WalkCommand::Stand) => WalkState::Standing,
            (WalkState::Standing, WalkCommand::Walk(step)) => WalkState::Starting(step),
            (WalkState::Starting(_), WalkCommand::Stand) => WalkState::Standing,
            (WalkState::Starting(_), WalkCommand::Walk(step)) => WalkState::Walking(step),
            (WalkState::Walking(_), WalkCommand::Stand) => WalkState::Stopping,
            (WalkState::Walking(_), WalkCommand::Walk(step)) => WalkState::Walking(step),
            (WalkState::Stopping, WalkCommand::Stand) => WalkState::Standing,
            (WalkState::Stopping, WalkCommand::Walk(step)) => WalkState::Walking(step),
            (WalkState::Standing, WalkCommand::Kick(..)) => WalkState::Starting(Step::zero()),
            (WalkState::Starting(_), WalkCommand::Kick(kick_variant, kick_side)) => {
                if kick_side == swing_side.opposite() {
                    WalkState::Kicking(kick_variant, kick_side, 0)
                } else {
                    WalkState::Walking(Step::zero())
                }
            }
            (WalkState::Walking(_), WalkCommand::Kick(kick_variant, kick_side)) => {
                if kick_side == swing_side.opposite() {
                    WalkState::Kicking(kick_variant, kick_side, 0)
                } else {
                    WalkState::Walking(Step::zero())
                }
            }
            (WalkState::Kicking(kick_variant, kick_side, step_i), WalkCommand::Stand) => {
                let num_steps = match kick_variant {
                    KickVariant::Forward => &kick_steps.forward,
                    KickVariant::Turn => &kick_steps.turn,
                    KickVariant::Side => &kick_steps.side,
                }
                .len();
                if step_i + 1 < num_steps {
                    WalkState::Kicking(kick_variant, kick_side, step_i + 1)
                } else {
                    WalkState::Stopping
                }
            }
            (WalkState::Kicking(kick_variant, kick_side, step_i), WalkCommand::Walk(step)) => {
                let num_steps = match kick_variant {
                    KickVariant::Forward => &kick_steps.forward,
                    KickVariant::Turn => &kick_steps.turn,
                    KickVariant::Side => &kick_steps.side,
                }
                .len();
                if step_i + 1 < num_steps {
                    WalkState::Kicking(kick_variant, kick_side, step_i + 1)
                } else {
                    WalkState::Walking(step)
                }
            }
            (
                WalkState::Kicking(current_kick_variant, current_kick_side, step_i),
                WalkCommand::Kick(next_kick_variant, next_kick_side),
            ) => {
                let num_steps = match current_kick_variant {
                    KickVariant::Forward => &kick_steps.forward,
                    KickVariant::Turn => &kick_steps.turn,
                    KickVariant::Side => &kick_steps.side,
                }
                .len();
                if step_i + 1 < num_steps {
                    WalkState::Kicking(current_kick_variant, current_kick_side, step_i + 1)
                } else if next_kick_side == swing_side.opposite() {
                    WalkState::Kicking(next_kick_variant, next_kick_side, 0)
                } else {
                    WalkState::Walking(Step::zero())
                }
            }
            (WalkState::Stopping, WalkCommand::Kick(kick_variant, kick_side)) => {
                if kick_side == swing_side.opposite() {
                    WalkState::Kicking(kick_variant, kick_side, 0)
                } else {
                    WalkState::Walking(Step::zero())
                }
            }
        }
    }
}
