use std::{convert::TryFrom, path::Path, time::UNIX_EPOCH};

use anyhow::Context;
use log::info;
use mlua::{Lua, LuaSerdeExt};
use serde_json::from_value;

use crate::{
    framework::{communication::configuration_directory::deserialize, Configuration},
    hardware::HardwareIds,
};

use super::{
    configuration::Configuration as SimulationConfiguration, recording::Recording, robot::Robot,
    state::State,
};

pub fn simulate<ConfigurationPath, RecordingPath>(
    configuration_path: ConfigurationPath,
    recording_path: RecordingPath,
) -> anyhow::Result<()>
where
    ConfigurationPath: AsRef<Path>,
    RecordingPath: AsRef<Path>,
{
    let configuration = SimulationConfiguration::read_from(configuration_path)
        .context("Failed to read simulation configuration")?;

    let configurations = robot_configurations_from_ids(&configuration.robot_ids)
        .context("Failed to get robot configurations from robot ids")?;
    let mut recording = Recording::from((&configuration, &configurations));
    let mut robots = configurations
        .into_iter()
        .map(|configuration| {
            let player_number = configuration.player_number;
            Robot::try_from(configuration).with_context(|| {
                format!("Failed to initialize robot with player number {player_number}")
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to initialize robots")?;
    let mut state = State::try_from(configuration).context("Failed to create simulation state")?;

    let lua = Lua::new();

    info!("Simulating...");
    loop {
        let frame_index = recording.frames.len();
        fill_globals_with(&lua, frame_index, &state, &robots)
            .context("Failed to fill Lua globals")?;

        let databases = match state
            .step(&lua, &mut robots)
            .context("Failed to step simulation")?
        {
            Some(databases) => databases,
            None => break,
        };
        recording.push_frame(&state, &robots, databases);
    }

    info!("Writing recording...");
    recording
        .write_to(recording_path)
        .context("Failed to write simulation recording")?;

    Ok(())
}

fn robot_configurations_from_ids(ids: &[String]) -> anyhow::Result<Vec<Configuration>> {
    ids.iter()
        .map(|id| HardwareIds {
            body_id: id.clone(),
            head_id: id.clone(),
        })
        .map(|hardware_ids| {
            let head_id = hardware_ids.head_id.clone();
            from_value::<Configuration>(
                deserialize("etc/configuration", hardware_ids)
                    .with_context(|| format!("Failed to deserialize for {head_id:?}"))?,
            )
            .with_context(|| format!("Failed to construct configuration for {head_id:?}"))
        })
        .collect::<Result<_, _>>()
        .context("Failed to read configurations")
}

fn fill_globals_with(
    lua: &Lua,
    frame_index: usize,
    state: &State,
    robots: &[Robot],
) -> anyhow::Result<()> {
    let globals = lua.globals();

    globals
        .set("frame_index", frame_index)
        .context("Failed to set frame_index")?;
    globals
        .set(
            "frame_seconds",
            state
                .now
                .duration_since(UNIX_EPOCH)
                .expect("Time ran backwards")
                .as_secs_f32(),
        )
        .context("Failed to set frame_seconds")?;
    globals
        .set(
            "state",
            lua.to_value(state)
                .context("Failed to convert state to Lua object")?,
        )
        .context("Failed to set state")?;
    globals
        .set(
            "robots",
            lua.to_value(robots)
                .context("Failed to convert robots to Lua object")?,
        )
        .context("Failed to set robots")?;

    Ok(())
}
