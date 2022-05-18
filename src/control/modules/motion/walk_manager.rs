use macros::{module, require_some};

use crate::types::{
    BodyMotionType, Motion, MotionCommand, MotionSelection, StepPlan, WalkAction, WalkCommand,
};

pub struct WalkManager;

#[module(control)]
#[input(data_type = StepPlan, path = step_plan)]
#[input(path = motion_command, data_type = MotionCommand)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[main_output(data_type = WalkCommand)]
impl WalkManager {}

impl WalkManager {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let step_plan = require_some!(context.step_plan);
        let motion_command = require_some!(context.motion_command);
        let motion_selection = require_some!(context.motion_selection);

        let action = if let (BodyMotionType::Walk, Motion::Walk { .. }) =
            (motion_selection.current_body_motion, motion_command.motion)
        {
            WalkAction::Walk
        } else {
            WalkAction::Stand
        };

        Ok(MainOutputs {
            walk_command: Some(WalkCommand {
                step: step_plan.step,
                action,
            }),
        })
    }
}
