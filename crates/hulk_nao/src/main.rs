#![recursion_limit = "256"]
use std::{fs::File, io::stdout, sync::Arc};

use color_eyre::{
    eyre::{Result, WrapErr},
    install,
};
use hardware::IdInterface;
use hardware_interface::HardwareInterface;
use serde_json::from_reader;
use tokio_util::sync::CancellationToken;

use crate::run::run;

mod camera;
mod double_buffered_reader;
mod hardware_interface;
mod hula;
mod hula_wrapper;
mod microphones;

include!(concat!(env!("OUT_DIR"), "/generated_framework.rs"));

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
    let keep_running = CancellationToken::new();
    ctrlc::set_handler({
        let keep_running = keep_running.clone();
        move || {
            keep_running.cancel();
        }
    })?;
    let file = File::open("etc/configuration/hardware.json")
        .wrap_err("failed to open hardware parameters")?;
    let hardware_parameters = from_reader(file).wrap_err("failed to parse hardware parameters")?;
    let hardware_interface = HardwareInterface::new(keep_running.clone(), hardware_parameters)
        .wrap_err("failed to create hardware interface")?;
    let ids = hardware_interface.get_ids();
    run(
        Arc::new(hardware_interface),
        Some("[::]:1337"),
        "etc/configuration",
        ids.body_id,
        ids.head_id,
        keep_running,
    )
}
