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
use hardware::IdInterface;
use hardware_interface::{HardwareInterface, Parameters as HardwareParameters};
use hulk::execution::run;
use serde_json::from_reader;
use tokio_util::sync::CancellationToken;

mod audio_parameter_deserializers;
mod camera;
mod double_buffered_reader;
mod hardware_interface;
mod hula;
mod hula_wrapper;
mod microphones;
mod speakers;

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

#[derive(Parser)]
struct Arguments {
    #[arg(short, long, default_value = "logs")]
    log_path: PathBuf,

    #[arg(short, long, default_value = "etc/parameters/framework.json")]
    framework_parameters_path: PathBuf,
}

fn main() -> Result<()> {
    setup_logger()?;
    install()?;
    let keep_running = CancellationToken::new();
    set_handler({
        let keep_running = keep_running.clone();
        move || {
            keep_running.cancel();
        }
    })?;

    let arguments = Arguments::parse();
    let framework_parameters_path = Path::new(&arguments.framework_parameters_path);

    let file =
        File::open(framework_parameters_path).wrap_err("failed to open framework parameters")?;
    let framework_parameters: FrameworkParameters =
        from_reader(file).wrap_err("failed to parse framework parameters")?;

    let file = File::open(framework_parameters.hardware_parameters)
        .wrap_err("failed to open hardware parameters")?;
    let hardware_parameters: HardwareParameters =
        from_reader(file).wrap_err("failed to parse hardware parameters")?;

    let hardware_interface = HardwareInterface::new(keep_running.clone(), hardware_parameters)
        .wrap_err("failed to create hardware interface")?;

    let ids = hardware_interface.get_ids();

    run(
        Arc::new(hardware_interface),
        framework_parameters.communication_addresses,
        framework_parameters.parameters_directory,
        arguments.log_path,
        ids,
        keep_running,
        framework_parameters.recording_intervals,
    )
}
