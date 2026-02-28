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
    ButtonEventMsgInterface, CameraInterface, FallDownStateInterface, IdInterface,
    LowCommandInterface, LowStateInterface, MicrophoneInterface, NetworkInterface, PathsInterface,
    RecordingInterface, SafeToExitSafeInterface, SpeakerInterface, TimeInterface,
    TransformMessageInterface,
};
use hula_types::hardware::Ids;
use serde_json::from_reader;
use tokio_util::sync::CancellationToken;

use crate::execution::run;
use crate::hardware_interface::{BoosterHardwareInterface, Parameters as HardwareParameters};

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
    IdInterface
    + LowStateInterface
    + LowCommandInterface
    + CameraInterface
    + FallDownStateInterface
    + ButtonEventMsgInterface
    + TransformMessageInterface
    + MicrophoneInterface
    + NetworkInterface
    + PathsInterface
    + RecordingInterface
    + SpeakerInterface
    + TimeInterface
    + SafeToExitSafeInterface
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
