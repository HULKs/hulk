use bevy::ecs::schedule::IntoSystemConfigs;

use crate::simulator::{AppExt, SimulatorPlugin};

pub fn run_scenario<M>(
    system: impl IntoSystemConfigs<M>,
    with_recording: bool,
) -> color_eyre::Result<()> {
    bevy::app::App::new()
        .add_plugins(SimulatorPlugin::default().with_recording(with_recording))
        .add_systems(bevy::app::Update, system)
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
