#![recursion_limit = "256"]
use std::{env::args, fs::File, io::stdout, sync::Arc};

use color_eyre::{
    eyre::{Result, WrapErr},
    install,
};
use ctrlc::set_handler;
use hardware::{IdInterface, PathsInterface};
use hardware_interface::{HardwareInterface, Parameters};
use hulk::run::run;
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

fn main() -> Result<()> {
    setup_logger()?;
    install()?;
    let hardware_parameters_path = args()
        .nth(1)
        .unwrap_or("etc/parameters/hardware.json".to_string());
    let keep_running = CancellationToken::new();
    set_handler({
        let keep_running = keep_running.clone();
        move || {
            keep_running.cancel();
        }
    })?;
    let file =
        File::open(hardware_parameters_path).wrap_err("failed to open hardware parameters")?;
    let hardware_parameters: Parameters =
        from_reader(file).wrap_err("failed to parse hardware parameters")?;
    let communication_addresses = hardware_parameters.communication_addresses.clone();
    let hardware_interface = HardwareInterface::new(keep_running.clone(), hardware_parameters)
        .wrap_err("failed to create hardware interface")?;
    let ids = hardware_interface.get_ids();
    let paths = hardware_interface.get_paths();
    run(
        Arc::new(hardware_interface),
        communication_addresses,
        paths.parameters,
        ids.body_id,
        ids.head_id,
        keep_running,
    )
}
