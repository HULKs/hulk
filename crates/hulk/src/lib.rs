#![recursion_limit = "256"]
use std::io::stdout;

use color_eyre::eyre::Result;

#[cfg(feature = "nao")]
pub mod nao;
#[cfg(feature = "webots")]
pub mod webots;

include!(concat!(env!("OUT_DIR"), "/generated_framework.rs"));

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
