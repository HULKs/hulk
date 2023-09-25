use std::{io::stdout, path::PathBuf, time::Instant};

use chrono::Local;
use clap::Parser;
use color_eyre::{eyre::Context, install, Result};
use fern::{Dispatch, InitError};
use log::LevelFilter;
use tokio_util::sync::CancellationToken;

mod cycler;
mod interfake;
mod robot;
mod server;
mod simulator;
mod state;

use hardware::{NetworkInterface, TimeInterface};

use crate::simulator::Simulator;

pub trait HardwareInterface: TimeInterface + NetworkInterface {}

include!(concat!(env!("OUT_DIR"), "/generated_code.rs"));

#[derive(Parser)]
enum Arguments {
    Run(RunArguments),
    Serve(ServeArguments),
}

#[derive(Parser)]
struct RunArguments {
    scenario_file: PathBuf,
}

#[derive(Parser)]
struct ServeArguments {
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

    let arguments = Arguments::parse();
    match arguments {
        Arguments::Run(arguments) => run(arguments),
        Arguments::Serve(arguments) => serve(arguments),
    }
}

fn run(arguments: RunArguments) -> Result<()> {
    let mut simulator = Simulator::try_new()?;
    simulator.execute_script(arguments.scenario_file)?;

    let start = Instant::now();
    simulator.run().wrap_err("failed to run simulation")?;
    let duration = Instant::now() - start;
    println!("Took {:.2} seconds", duration.as_secs_f32());

    Ok(())
}

fn serve(arguments: ServeArguments) -> Result<()> {
    let keep_running = CancellationToken::new();
    {
        let keep_running = keep_running.clone();
        ctrlc::set_handler(move || {
            println!("Cancelling...");
            keep_running.cancel();
        })?;
    }

    server::run(
        Some(arguments.listen_address),
        keep_running,
        arguments.scenario_file,
    )
}
