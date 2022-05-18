use anyhow::Result;
use macros::{module, require_some};

use crate::types::Actions;
use crate::types::MotionCommand;
use crate::types::WorldState;

pub struct Behavior {}

#[module(control)]
#[input(path = world_state, data_type = WorldState)]
#[parameter(path = control.behavior.injected_motion_command, data_type = Option<MotionCommand>)]
#[main_output(data_type = MotionCommand)]
impl Behavior {}

impl Behavior {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let actions = vec![
            Actions::Unstiff,
            Actions::SitDown,
            Actions::Penalize,
            Actions::FallSafely,
            Actions::StandUp,
            Actions::Stand,
            Actions::WalkToPose,
        ];

        let mut chosen_action = Actions::Penalize;
        let world_state = require_some!(context.world_state);
        for action in actions {
            if action.is_available(world_state) {
                chosen_action = action;
                break;
            }
        }

        Ok(MainOutputs {
            motion_command: Some(
                context
                    .injected_motion_command
                    .unwrap_or(chosen_action.execute(world_state)),
            ),
        })
    }
}
