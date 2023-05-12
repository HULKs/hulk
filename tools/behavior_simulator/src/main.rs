use std::{io::stdout, path::PathBuf};

use chrono::Local;
use clap::Parser;
use color_eyre::{install, Result};
use fern::{Dispatch, InitError};
use log::LevelFilter;
use tokio_util::sync::CancellationToken;

mod cycler;
#[allow(dead_code)]
mod fake_data;
mod interfake;
mod robot;
mod server;
mod simulator;
mod state;

mod structs {
    include!(concat!(env!("OUT_DIR"), "/generated_structs.rs"));
}

#[derive(Parser)]
struct Arguments {
    #[arg(short, long, default_value = "[::]:1337")]
    listen_address: String,
    scenario_file: PathBuf,
}

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

    let arguments = Arguments::parse();

    server::run(
        Some(arguments.listen_address),
        keep_running,
        arguments.scenario_file,
    )
}
