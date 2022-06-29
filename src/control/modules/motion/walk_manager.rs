use macros::{module, require_some};

use crate::types::{MotionCommand, MotionSelection, MotionType, Step, WalkCommand};

pub struct WalkManager;

#[module(control)]
#[input(data_type = Step, path = step_plan)]
#[input(path = motion_command, data_type = MotionCommand)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[main_output(data_type = WalkCommand)]
impl WalkManager {}

impl WalkManager {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let motion_command = require_some!(context.motion_command);
        let motion_selection = require_some!(context.motion_selection);

        let command = match (motion_command, motion_selection.current_motion) {
            (MotionCommand::Walk { .. }, MotionType::Walk) => match context.step_plan {
                Some(step) => WalkCommand::Walk(*step),
                None => WalkCommand::Stand,
            },
            (
                MotionCommand::InWalkKick {
                    kick, kicking_side, ..
                },
                MotionType::Walk,
            ) => WalkCommand::Kick(*kick, *kicking_side),
            _ => WalkCommand::Stand,
        };

        Ok(MainOutputs {
            walk_command: Some(command),
        })
    }
}
