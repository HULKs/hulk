use bevy::app::{App, Plugins};
use clap::Parser;
use color_eyre::Result;

use crate::simulator::{AppExt, SimulatorPlugin};

#[derive(Parser)]
pub struct Arguments {
    /// Just run the simulation, don't serve the result
    #[arg(short, long)]
    pub run: bool,
}

pub fn run_scenario<M>(plugin: impl Plugins<M>, with_recording: bool) -> Result<()> {
    let args = Arguments::try_parse().unwrap();

    App::new()
        .add_plugins(SimulatorPlugin::default().with_recording(!args.run && with_recording))
        .add_plugins(plugin)
        .run_to_completion()
}
