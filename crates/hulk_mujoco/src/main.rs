#![recursion_limit = "256"]
use std::{env::args, fs::File, io::stdout, path::Path, sync::Arc};

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
        .level_for("ort", log::LevelFilter::Warn)
        .chain(stdout())
        .apply()?;
    Ok(())
}

pub trait HardwareInterface:
    IdInterface
    + LowStateInterface
    + LowCommandInterface
    + CameraInterface
    + MicrophoneInterface
    + NetworkInterface
    + PathsInterface
    + RecordingInterface
    + SpeakerInterface
    + TimeInterface
{
}

include!(concat!(env!("OUT_DIR"), "/generated_code.rs"));

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
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

    if !Path::new("logs").exists() {
        std::fs::create_dir("logs").wrap_err("failed to create logs directory")?;
    }

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
