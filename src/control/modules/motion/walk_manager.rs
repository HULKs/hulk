use macros::{module, require_some};

use crate::types::{
    BodyMotionType, GroundContact, Motion, MotionCommand, MotionSelection, StepPlan, WalkCommand,
};

pub struct WalkManager;

#[module(control)]
#[input(data_type = StepPlan, path = step_plan)]
#[input(path = motion_command, data_type = MotionCommand)]
#[input(path = ground_contact, data_type = GroundContact)]
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

        let command = if let (BodyMotionType::Walk, Motion::Walk { .. }, Some(StepPlan { step })) = (
            motion_selection.current_body_motion,
            motion_command.motion,
            context.step_plan,
        ) {
            WalkCommand::Walk(*step)
        } else {
            WalkCommand::Stand
        };

        Ok(MainOutputs {
            walk_command: Some(command),
        })
    }
}
