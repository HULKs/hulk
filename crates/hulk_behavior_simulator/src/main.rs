use std::{io::stdout, path::PathBuf, time::Instant};

use chrono::Local;
use clap::Parser;
use color_eyre::{eyre::Context, install, Result};
use fern::{Dispatch, InitError};
use log::LevelFilter;
use tokio_util::sync::CancellationToken;

use hulk_behavior_simulator::{server, simulator::Simulator};

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
        Arguments::Run(..) => run(),
        Arguments::Serve(arguments) => serve(arguments),
    }
}

fn run() -> Result<()> {
    let mut simulator = Simulator::default();

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

    let mut simulator = Simulator::default();
    let start = Instant::now();
    simulator.run().wrap_err("failed to run simulation")?;
    let duration = Instant::now() - start;
    println!("Took {:.2} seconds", duration.as_secs_f32());

    server::run(
        simulator.frames,
        arguments.listen_address)
        keep_running,
    )
}
