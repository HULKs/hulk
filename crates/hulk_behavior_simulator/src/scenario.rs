use bevy::{
    app::{App, Update},
    ecs::schedule::IntoSystemConfigs,
};
use color_eyre::Result;

use crate::simulator::{AppExt, SimulatorPlugin};

use clap::Parser;

#[derive(Parser)]
struct Arguments {
    /// Just run the simulation, don't serve the result
    #[arg(short, long)]
    run: bool,
}

pub fn run_scenario<M>(system: impl IntoSystemConfigs<M>, with_recording: bool) -> Result<()> {
    let args = Arguments::parse();

    App::new()
        .add_plugins(SimulatorPlugin::default().with_recording(!args.run && with_recording))
        .add_systems(Update, system)
        .run_to_completion()
}

#[macro_export]
macro_rules! scenario {
    ($name:ident) => {
        fn main() -> color_eyre::Result<()> {
            hulk_behavior_simulator::scenario::run_scenario($name, true)
        }

        #[cfg(test)]
        mod test {
            use super::$name as scenario;

            #[test]
            fn $name() -> color_eyre::Result<()> {
                hulk_behavior_simulator::scenario::run_scenario(scenario, false)
            }
        }
    };
}
