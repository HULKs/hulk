use std::path::PathBuf;

use anyhow::Context;
use hulk::{behavior_simulator::simulate, setup_logger};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Arguments {
    /// Path to configuration file
    configuration_path: PathBuf,
    /// Path to recording file
    recording_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    setup_logger()?;
    let arguments = Arguments::from_args();
    simulate(arguments.configuration_path, arguments.recording_path)
        .context("simulate(arguments.configuration_path)")?;
    Ok(())
}
