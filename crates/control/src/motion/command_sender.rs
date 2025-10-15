use std::f32::consts::PI;

use approx::abs_diff_eq;
use booster::{CommandType, LowCommand, MotorCommand};
use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use hardware::LowCommandInterface;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct CommandSender {
    time_index: f32,
    motor_index: usize,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl CommandSender {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            time_index: 0.0,
            motor_index: 0,
        })
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl LowCommandInterface>,
    ) -> Result<MainOutputs> {
        let motor_commands =
            Self::generate_random_motor_commands(self.motor_index, self.time_index);

        self.time_index += PI / 100.0;
        if abs_diff_eq!(self.time_index % (8.0 * PI), 0.0, epsilon = 0.001) {
            self.motor_index = (self.motor_index + 1) % 22;
            self.time_index = 0.0;
        }

        let low_command = LowCommand {
            command_type: CommandType::Serial,
            motor_commands: motor_commands.to_vec(),
        };

        context
            .hardware_interface
            .write_low_command(low_command)
            .wrap_err("failed to write to actuators")?;

        Ok(MainOutputs {})
    }

    fn generate_random_motor_commands(motor_index: usize, time_index: f32) -> [MotorCommand; 22] {
        let mut motor_commands: [MotorCommand; 22] = [MotorCommand {
            position: 0.0,
            velocity: 0.0,
            torque: 0.0,
            kp: 45.0,
            kd: 0.2,
            weight: 1.0,
        }; 22];
        motor_commands[motor_index] = MotorCommand {
            position: time_index.sin(),
            velocity: time_index.sin(),
            torque: 1.0,
            kp: 25.0,
            kd: 0.3,
            weight: 1.0,
        };
        motor_commands
    }
}
