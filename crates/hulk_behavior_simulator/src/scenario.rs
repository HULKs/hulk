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

#[macro_export]
macro_rules! scenario {
    ($name:ident, $plugin:expr) => {
        fn main() -> color_eyre::Result<()> {
            use clap::Parser;
            use hulk_behavior_simulator::simulator::{AppExt, SimulatorPlugin};

            let args = hulk_behavior_simulator::scenario::Arguments::parse();

            App::new()
                .add_plugins(SimulatorPlugin::default().with_recording(!args.run))
                .add_plugins($plugin)
                .run_to_completion()
        }

        #[cfg(test)]
        mod test {
            use super::*;

            paste::item! {
                #[test]
                fn [< $name _test >]() -> color_eyre::Result<()> {
                    use hulk_behavior_simulator::simulator::{AppExt, SimulatorPlugin};

                    App::new()
                        .add_plugins(SimulatorPlugin::default())
                        .add_plugins($plugin)
                        .run_to_completion()
                }
            }
        }
    };
}
