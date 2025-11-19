use color_eyre::Result;
use context_attribute::context;

use framework::MainOutput;
use linear_algebra::vector;
use serde::{Deserialize, Serialize};
use types::{
    motion_command::{HeadMotion, MotionCommand},
    parameters::RemoteControlParameters,
};

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    remote_control_parameters: Parameter<RemoteControlParameters, "remote_control_parameters">,
}

#[derive(Deserialize, Serialize)]
pub struct RemoteControl {}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motion_command: MainOutput<MotionCommand>,
}

impl RemoteControl {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(RemoteControl {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let motion_command = MotionCommand::WalkWithVelocity {
            head: HeadMotion::Center,
            velocity: vector!(
                context.remote_control_parameters.walk.forward,
                context.remote_control_parameters.walk.left,
            ),
            angular_velocity: context.remote_control_parameters.walk.turn,
        };

        Ok(MainOutputs {
            motion_command: motion_command.into(),
        })
    }
}
