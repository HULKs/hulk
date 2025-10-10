#![recursion_limit = "256"]
use std::{env::args, fs::File, io::stdout, sync::Arc};

use color_eyre::{
    eyre::{Result, WrapErr},
    install,
};
use ctrlc::set_handler;
use framework::Parameters as FrameworkParameters;
use hardware::{
    CameraInterface, IdInterface, LowCommandInterface, LowStateInterface, MicrophoneInterface,
    NetworkInterface, PathsInterface, RecordingInterface, SpeakerInterface, TimeInterface,
};
use hula_types::hardware::Ids;
use serde_json::from_reader;
use tokio_util::sync::CancellationToken;

use crate::execution::run;
use crate::hardware_interface::{MujocoHardwareInterface, Parameters as HardwareParameters};

mod hardware_interface;

pub fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}  {:<18}  {:>5}  {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(stdout())
        .apply()?;
    Ok(())
}

pub trait HardwareInterface:
    CameraInterface
    + IdInterface
    + LowStateInterface
    + LowCommandInterface
    + MicrophoneInterface
    + NetworkInterface
    + PathsInterface
    + RecordingInterface
    + SpeakerInterface
    + TimeInterface
{
}

include!(concat!(env!("OUT_DIR"), "/generated_code.rs"));

fn main() -> Result<()> {
    setup_logger()?;
    install()?;
    let framework_parameters_path = args()
        .nth(1)
        .unwrap_or("etc/parameters/framework.json".to_string());
    let keep_running = CancellationToken::new();
    set_handler({
        let keep_running = keep_running.clone();
        move || {
            keep_running.cancel();
        }
    })?;

    let file =
        File::open(framework_parameters_path).wrap_err("failed to open framework parameters")?;
    let mut framework_parameters: FrameworkParameters =
        from_reader(file).wrap_err("failed to parse framework parameters")?;

    let file = File::open(framework_parameters.hardware_parameters)
        .wrap_err("failed to open hardware parameters")?;
    let hardware_parameters: HardwareParameters =
        from_reader(file).wrap_err("failed to parse hardware parameters")?;

    if framework_parameters.communication_addresses.is_none() {
        let fallback = "127.0.0.1:1337";
        println!("framework.json disabled communication, falling back to {fallback}");
        framework_parameters.communication_addresses = Some(fallback.to_string());
    }

    let hardware_interface =
        MujocoHardwareInterface::new(keep_running.clone(), hardware_parameters)?;

    run(
        Arc::new(hardware_interface),
        framework_parameters.communication_addresses,
        framework_parameters.parameters_directory,
        "logs",
        Ids {
            body_id: "K1_BODY".to_string(),
            head_id: "K1_HEAD".to_string(),
        },
        keep_running,
        framework_parameters.recording_intervals,
    )
}

#[cfg(test)]
mod mujoco_test {
    use std::{
        env::args,
        f32::{self, consts::PI},
        fs::File,
    };

    use approx::abs_diff_eq;
    use booster_low_level_interface::{CommandType, LowCommand, MotorCommand};
    use ctrlc::set_handler;
    use framework::Parameters as FrameworkParameters;
    use hardware::{LowCommandInterface, LowStateInterface};
    use serde_json::from_reader;
    use tokio_util::sync::CancellationToken;

    use crate::hardware_interface::{MujocoHardwareInterface, Parameters as HardwareParameters};

    #[tokio::test]
    async fn test_mujoco_connection() {
        let framework_parameters_path = args()
            .nth(1)
            .unwrap_or("etc/parameters/framework.json".to_string());
        let keep_running = CancellationToken::new();
        set_handler({
            let keep_running = keep_running.clone();
            move || {
                keep_running.cancel();
            }
        })
        .expect("could not set handler");

        let file =
            File::open(framework_parameters_path).expect("failed to open framework parameters");
        let framework_parameters: FrameworkParameters =
            from_reader(file).expect("failed to parse framework parameters");

        let hardware_parameters_file = File::open(framework_parameters.hardware_parameters)
            .expect("failed to open hardware parameters");
        let hardware_parameters: HardwareParameters =
            from_reader(hardware_parameters_file).expect("failed to parse hardware parameters");

        let hardware_interface =
            MujocoHardwareInterface::new(keep_running.clone(), hardware_parameters)
                .expect("failed to create hardware interface");

        let tokio_runtime_handle = tokio::runtime::Handle::current();

        tokio_runtime_handle
            .spawn_blocking(move || {
                let mut time_index: f32 = 0.0;
                let mut motor_index: usize = 0;
                loop {
                    let low_state = hardware_interface
                        .read_low_state()
                        .expect("failed to read low state");

                    let motor_commands = generate_random_motor_commands(motor_index, time_index);

                    hardware_interface
                        .write_low_command(LowCommand {
                            command_type: CommandType::Serial,
                            motor_commands: motor_commands.to_vec(),
                        })
                        .expect("failed to write low command");

                    time_index += PI / 100.0;
                    if abs_diff_eq!(time_index % (8.0 * PI), 0.0, epsilon = 0.001) {
                        motor_index = (motor_index + 1) % 22;
                        time_index = 0.0;
                    }
                }
            })
            .await
            .expect("failed to join");
    }

    fn generate_random_motor_commands(motor_index: usize, time_index: f32) -> [MotorCommand; 22] {
        let mut motor_commands: [MotorCommand; 22] = [MotorCommand {
            position: 0.0,
            velocity: 0.0,
            torque: 0.0,
            kp: 45.0,
            kd: 0.1,
            weight: 1.0,
        }; 22];
        motor_commands[motor_index] = MotorCommand {
            position: time_index.sin(),
            velocity: time_index.sin(),
            torque: 1.0,
            kp: 10.0,
            kd: 1.0,
            weight: 1.0,
        };
        motor_commands
    }
}
