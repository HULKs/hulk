use std::time::SystemTime;

use anyhow::Context;
use nalgebra::Isometry2;
use spl_network::SplMessage;

use crate::{
    framework::{future_queue::Data, Configuration, PerceptionDatabases},
    spl_network::MainOutputs,
    types::{BallPosition, FallState, PrimaryState, SensorData},
};

use super::{
    modules::{
        behavior::Behavior, path_planner::PathPlanner, world_state_composer::WorldStateComposer,
    },
    Database, PersistentState,
};

pub struct BehaviorCycler {
    persistent_state: PersistentState,

    world_state_composer: WorldStateComposer,
    behavior: Behavior,
    path_planner: PathPlanner,
}

impl BehaviorCycler {
    pub fn new(configuration: &Configuration) -> anyhow::Result<Self> {
        Ok(Self {
            persistent_state: Default::default(),

            world_state_composer: WorldStateComposer::run_new(configuration)
                .context("Failed to initialize module WorldStateComposer")?,
            behavior: Behavior::run_new(configuration)
                .context("Failed to initialize module Behavior")?,
            path_planner: PathPlanner::run_new(configuration)
                .context("Failed to initialize module PathPlanner")?,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_cycle(
        &mut self,
        configuration: &Configuration,
        cycle_start_time: SystemTime,
        ball_position: BallPosition,
        fall_state: FallState,
        robot_to_field: Isometry2<f32>,
        sensor_data: SensorData,
        primary_state: PrimaryState,
        broadcasted_spl_messages: Vec<SplMessage>,
    ) -> anyhow::Result<Database> {
        let mut control_database = Database::default();
        control_database.main_outputs.ball_position = Some(ball_position);
        control_database.main_outputs.fall_state = Some(fall_state);
        control_database.main_outputs.robot_to_field = Some(robot_to_field);
        control_database.main_outputs.sensor_data = Some(sensor_data);
        control_database.main_outputs.primary_state = Some(primary_state);

        let historic_databases = Default::default();
        let mut perception_databases = PerceptionDatabases::default();
        perception_databases.update(
            cycle_start_time,
            (vec![], None),
            (
                broadcasted_spl_messages
                    .into_iter()
                    .map(|message| Data {
                        timestamp: cycle_start_time,
                        data: MainOutputs {
                            game_controller_state_message: None,
                            spl_message: Some(message),
                        },
                    })
                    .collect(),
                None,
            ),
            (vec![], None),
            (vec![], None),
        );

        let subscribed_additional_outputs = Default::default();
        let changed_parameters = Default::default();

        self.world_state_composer
            .run_cycle(
                cycle_start_time,
                &mut control_database,
                &historic_databases,
                &perception_databases,
                configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )
            .context("Failed to run cycle of module WorldStateComposer")?;
        self.behavior
            .run_cycle(
                cycle_start_time,
                &mut control_database,
                &historic_databases,
                &perception_databases,
                configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )
            .context("Failed to run cycle of module Behavior")?;
        self.path_planner
            .run_cycle(
                cycle_start_time,
                &mut control_database,
                &historic_databases,
                &perception_databases,
                configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )
            .context("Failed to run cycle of module PathPlanner")?;

        Ok(control_database)
    }
}
