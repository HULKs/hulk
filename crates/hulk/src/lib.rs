#![recursion_limit = "256"]
use std::io::stdout;

use color_eyre::eyre::Result;

mod audio;
mod control;
pub mod cyclers;
#[cfg(feature = "nao")]
pub mod nao;
mod perception_databases;
mod spl_network;
mod structs;
mod vision;
#[cfg(feature = "webots")]
pub mod webots;

pub fn setup_logger(is_verbose: bool) -> Result<(), fern::InitError> {
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
        .level(if is_verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .chain(stdout())
        .apply()?;
    Ok(())
}
