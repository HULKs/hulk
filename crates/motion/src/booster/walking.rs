use booster_sdk::types::RobotMode;
use color_eyre::Result;
use context_attribute::context;
use hardware::{HighLevelInterface, MotionRuntimeInteface, TimeInterface};
use serde::{Deserialize, Serialize};
use types::{motion_command::MotionCommand, motion_runtime::MotionRuntime, step::Step};

#[derive(Deserialize, Serialize)]
pub struct BoosterWalking {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    robot_mode: RequiredInput<Option<RobotMode>, "WorldState", "robot_mode?">,

    motion_command: Input<MotionCommand, "WorldState", "motion_command">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl BoosterWalking {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl HighLevelInterface + MotionRuntimeInteface + TimeInterface>,
    ) -> Result<MainOutputs> {
        if context.hardware_interface.get_motion_runtime_type()? != MotionRuntime::Booster
            || !matches!(context.robot_mode, RobotMode::Walking)
        {
            return Ok(MainOutputs {});
        }

        match context.motion_command {
            MotionCommand::WalkWithVelocity {
                velocity,
                angular_velocity,
                ..
            } => move_robot(
                &context,
                Step {
                    forward: velocity.x(),
                    left: velocity.y(),
                    turn: *angular_velocity,
                },
            ),
            MotionCommand::Stand { .. } => move_robot(&context, Step::ZERO),
            _ => move_robot(&context, Step::ZERO),
        };

        Ok(MainOutputs {})
    }
}

fn move_robot(
    context: &CycleContext<impl HighLevelInterface + MotionRuntimeInteface + TimeInterface>,
    step: Step,
) {
    let _ = context
        .hardware_interface
        .move_robot(step)
        .inspect_err(|err| log::error!("{err:?}"));
}
