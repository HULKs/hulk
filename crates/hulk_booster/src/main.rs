#![recursion_limit = "256"]
use std::{
    fs::File,
    io::stdout,
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::Parser;
use color_eyre::{
    eyre::{Result, WrapErr},
    install,
};
use ctrlc::set_handler;
use framework::Parameters as FrameworkParameters;
use hardware::{
    ButtonEventMsgInterface, CameraInterface, FallDownStateInterface, HighLevelInterface,
    IdInterface, LightControlInterface, LowCommandInterface, LowStateInterface,
    MicrophoneInterface, MotionRuntimeInterface, NetworkInterface, OdometerInterface,
    PathsInterface, RecordingInterface, SimulatorInterface, SpeakerInterface, TimeInterface,
    VisualKickInterface,
};
use serde_json::from_reader;
use tokio_util::sync::CancellationToken;

use crate::{
    execution::run,
    hardware_interface::{BoosterHardwareInterface, Parameters as HardwareParameters},
};

mod audio_parameter_deserializers;
mod hardware_interface;
mod latest_receiver;
mod microphones;
mod x5_receiver;

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
        .level(log::LevelFilter::Info)
        .level_for("rustdds", log::LevelFilter::Error)
        .level_for("booster_sdk", log::LevelFilter::Error)
        .level_for("ort", log::LevelFilter::Warn)
        .chain(stdout())
        .apply()?;
    Ok(())
}

pub trait HardwareInterface:
    IdInterface
    + LowStateInterface
    + LowCommandInterface
    + VisualKickInterface
    + CameraInterface
    + FallDownStateInterface
    + ButtonEventMsgInterface
    + MicrophoneInterface
    + NetworkInterface
    + PathsInterface
    + RecordingInterface
    + SpeakerInterface
    + TimeInterface
    + SimulatorInterface
    + HighLevelInterface
    + MotionRuntimeInterface
    + OdometerInterface
    + LightControlInterface
{
}

include!(concat!(env!("OUT_DIR"), "/generated_code.rs"));

#[derive(Parser)]
struct Arguments {
    #[arg(short, long, default_value = "logs")]
    log_path: PathBuf,

    #[arg(short, long, default_value = "etc/parameters/framework.json")]
    framework_parameters_path: PathBuf,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    setup_logger()?;
    install()?;

    let arguments = Arguments::parse();
    let framework_parameters_path = Path::new(&arguments.framework_parameters_path);

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
        log::warn!("framework.json disabled communication, falling back to {fallback}");
        framework_parameters.communication_addresses = Some(fallback.to_string());
    }

    let runtime_handle = tokio::runtime::Handle::current();
    let hardware_interface =
        BoosterHardwareInterface::new(runtime_handle, keep_running.clone(), hardware_parameters)
            .await?;
    let ids = hardware_interface.get_ids();

    run(
        Arc::new(hardware_interface),
        framework_parameters.communication_addresses,
        framework_parameters.parameters_directory,
        "logs",
        ids,
        keep_running,
        framework_parameters.recording_intervals,
    )
}
