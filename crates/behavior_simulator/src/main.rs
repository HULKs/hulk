use std::io::stdout;

use chrono::Local;
use color_eyre::{install, Result};
use fern::{Dispatch, InitError};
use log::LevelFilter;
use tokio_util::sync::CancellationToken;

mod cycler;
mod interfake;
mod robot;
mod server;
mod simulator;
mod state;

fn setup_logger(is_verbose: bool) -> Result<(), InitError> {
    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}  {:<18}  {:>5}  {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(if is_verbose {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        })
        .chain(stdout())
        .apply()?;
    Ok(())
}

fn main() -> Result<()> {
    setup_logger(true)?;
    install()?;
    let keep_running = CancellationToken::new();
    {
        let keep_running = keep_running.clone();
        ctrlc::set_handler(move || {
            println!("Cancelling...");
            keep_running.cancel();
        })?;
    }

    server::run(keep_running)
}
