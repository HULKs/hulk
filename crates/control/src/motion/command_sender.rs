use booster::LowCommand;
use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::MainOutput;
use hardware::{LowCommandInterface, TimeInterface};
use serde::{Deserialize, Serialize};
use types::{joints::Joints, parameters::MotorCommandParameters};

#[derive(Deserialize, Serialize)]
pub struct CommandSender {
    time_index: f32,
    motor_index: usize,
    filtered_target_joint_positions: Joints,
}

#[context]
pub struct CreationContext {
    prepare_motor_command_parameters: Parameter<MotorCommandParameters, "prepare_motor_command">,
}

#[context]
pub struct CycleContext {
    target_joint_positions: Input<Joints, "target_joint_positions">,

    walk_motor_command_parameters: Parameter<MotorCommandParameters, "common_motor_command">,
    _prepare_motor_command_parameters: Parameter<MotorCommandParameters, "prepare_motor_command">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub low_command: MainOutput<LowCommand>,
}

impl CommandSender {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            time_index: 0.0,
            motor_index: 0,
            filtered_target_joint_positions: context
                .prepare_motor_command_parameters
                .default_positions,
        })
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl LowCommandInterface + TimeInterface>,
    ) -> Result<MainOutputs> {
        self.filtered_target_joint_positions =
            self.filtered_target_joint_positions * 0.8 + *context.target_joint_positions * 0.2;

        let walk_low_command = LowCommand::new(
            &self.filtered_target_joint_positions,
            context.walk_motor_command_parameters,
        );

        context
            .hardware_interface
            .write_low_command(walk_low_command.clone())
            .wrap_err("failed to write to actuators")?;

        Ok(MainOutputs {
            low_command: walk_low_command.into(),
        })
    }
}
