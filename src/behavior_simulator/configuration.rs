use std::{fs::File, path::Path, time::Duration};

use anyhow::Context;
use mlua::Lua;
use nalgebra::Isometry2;
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use spl_network::GameState;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Configuration {
    pub time_step: Duration,
    pub robot_ball_bounce_radius: f32,
    pub ball_velocity_decay_factor: f32,
    pub maximum_walk_angle_per_second: f32,
    pub maximum_walk_translation_distance_per_second: f32,
    pub robot_ids: Vec<String>,
    pub rules: Vec<Rule>,
}

impl Configuration {
    pub fn read_from<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = File::open(&path)
            .with_context(|| format!("Failed to open configuration file {:?}", path.as_ref()))?;
        from_reader(file).with_context(|| {
            format!(
                "Failed to read and parse configuration file {:?}",
                path.as_ref()
            )
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Rule {
    pub event: String,
    pub action: Action,
}

impl Rule {
    pub fn is_triggered(&self, lua: &Lua) -> anyhow::Result<bool> {
        lua.load(&self.event)
            .eval()
            .with_context(|| format!("Failed to evaluate {:?}", self.event))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Action {
    StopSimulation,
    SetGameState {
        game_state: GameState,
    },
    SetPenalized {
        robot_index: usize,
        is_penalized: bool,
    },
    SetRobotToField {
        robot_index: usize,
        robot_to_field: Isometry2<f32>,
    },
}
