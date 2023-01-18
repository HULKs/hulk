use std::{fs::File, path::Path, time::SystemTime};

use anyhow::Context;
use nalgebra::{Isometry2, Point2, UnitComplex, Vector2};
use serde::Serialize;
use serde_json::to_writer;
use spl_network::SplMessage;
use types::FilteredGameState;

use crate::{control::Database, framework::Configuration};

use super::{
    configuration::Configuration as SimulationConfiguration, robot::Robot as RobotState,
    state::State,
};

#[derive(Serialize)]
pub struct Recording {
    pub simulation_configuration: SimulationConfiguration,
    pub robot_configurations: Vec<Configuration>,
    pub frames: Vec<Frame>,
}

impl From<(&SimulationConfiguration, &Vec<Configuration>)> for Recording {
    fn from(
        (simulation_configuration, robot_configurations): (
            &SimulationConfiguration,
            &Vec<Configuration>,
        ),
    ) -> Self {
        Self {
            simulation_configuration: simulation_configuration.clone(),
            robot_configurations: robot_configurations.clone(),
            frames: vec![],
        }
    }
}

impl Recording {
    pub fn push_frame(&mut self, state: &State, robots: &[RobotState], databases: Vec<Database>) {
        self.frames.push(Frame {
            now: state.now,
            filtered_game_state: state.filtered_game_state,
            ball_position: state.ball_position,
            ball_velocity: state.ball_velocity,
            broadcasted_spl_message_counter: state.broadcasted_spl_message_counter,
            broadcasted_spl_messages: state.broadcasted_spl_messages.clone(),
            robots: robots
                .iter()
                .zip(databases)
                .map(|(robot, database)| Robot {
                    is_penalized: robot.is_penalized,
                    robot_to_field: robot.robot_to_field,
                    head_yaw: robot.head_yaw,
                    database,
                })
                .collect(),
        });
    }

    pub fn write_to(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let file = File::create(&path)
            .with_context(|| format!("Failed to create recording file {:?}", path.as_ref()))?;
        to_writer(file, &self).with_context(|| {
            format!(
                "Failed to read and parse recording file {:?}",
                path.as_ref()
            )
        })?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct Frame {
    pub now: SystemTime,
    pub filtered_game_state: FilteredGameState,
    pub ball_position: Point2<f32>,
    pub ball_velocity: Vector2<f32>,
    pub broadcasted_spl_message_counter: usize,
    pub broadcasted_spl_messages: Vec<SplMessage>,
    pub robots: Vec<Robot>,
}

#[derive(Serialize)]
pub struct Robot {
    pub is_penalized: bool,
    pub robot_to_field: Isometry2<f32>,
    pub head_yaw: UnitComplex<f32>,
    pub database: Database,
}
