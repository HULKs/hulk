use std::{fs::File, path::Path, time::Duration};

use anyhow::Context;
use mlua::Lua;
use nalgebra::{Isometry2, Point2, Vector2};
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use spl_network::SetPlay;
use types::FilteredGameState;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Configuration {
    pub time_step: Duration,
    pub robot_ball_bounce_radius: f32,
    pub ball_velocity_decay_factor: f32,
    pub maximum_field_of_view_angle: f32,
    pub maximum_field_of_view_distance: f32,
    pub maximum_walk_angle_per_second: f32,
    pub maximum_walk_translation_distance_per_second: f32,
    pub robot_ids: Vec<String>,
    pub rules: Vec<Rule>,
}

impl Configuration {
    pub fn read_from(path: impl AsRef<Path>) -> anyhow::Result<Self> {
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
    SetBallIsFree {
        ball_is_free: bool,
    },
    SetBallPosition {
        position: Point2<f32>,
    },
    SetBallVelocity {
        velocity: Vector2<f32>,
    },
    SetFilteredGameState {
        filtered_game_state: FilteredGameState,
    },
    SetPenalized {
        robot_index: usize,
        is_penalized: bool,
    },
    SetRobotToField {
        robot_index: usize,
        robot_to_field: Isometry2<f32>,
    },
    SetSetPlay {
        set_play: Option<SetPlay>,
    },
}
