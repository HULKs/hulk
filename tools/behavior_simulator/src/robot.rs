use std::{collections::BTreeMap, convert::Into, sync::Arc, time::SystemTime};

use color_eyre::{eyre::Context, Result};
use communication::server::parameters::directory::deserialize;
use control::localization::generate_initial_pose;
use spl_network_messages::PlayerNumber;
use structs::Configuration;
use types::messages::IncomingMessage;

use crate::{
    cycler::{BehaviorCycler, Database},
    interfake::Interfake,
};

pub struct Robot {
    pub interface: Arc<Interfake>,
    pub cycler: BehaviorCycler<Interfake>,

    pub database: Database,
    pub configuration: Configuration,

    pub penalized: bool,
}

impl Robot {
    pub fn try_new(player_number: PlayerNumber) -> Result<Self> {
        let interface: Arc<_> = Interfake::default().into();

        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let mut configuration: Configuration = runtime.block_on(async {
            deserialize(
                "etc/configuration",
                &format!("behavior_simulator{}", Into::<usize>::into(player_number)),
                &format!("behavior_simulator{}", Into::<usize>::into(player_number)),
            )
            .await
            .wrap_err("could not load initial parameters")
        })?;
        configuration.player_number = player_number;

        let cycler = BehaviorCycler::new(interface.clone(), Default::default(), &configuration)
            .context("failed to create cycler")?;

        let mut database = Database::default();

        database.main_outputs.robot_to_field = Some(generate_initial_pose(
            &configuration.localization.initial_poses[player_number],
            &configuration.field_dimensions,
        ));

        Ok(Self {
            interface,
            cycler,

            database,
            configuration,

            penalized: false,
        })
    }

    pub fn cycle(&mut self, messages: BTreeMap<SystemTime, Vec<&IncomingMessage>>) -> Result<()> {
        // Inputs to consider:
        // [x] ball position
        // [ ] fall state
        // [x] game controller state
        // [x] robot to field
        // [ ] cycle time
        // [x] messages
        // [ ] filtered game state
        // [ ] penalty shot direction
        // [x] team ball
        // [ ] has ground contact
        // [ ] obstacles
        // [ ] primary state
        // [x] role
        // [ ] world state

        // config:
        // forced role
        // player number
        // spl network
        // behavior

        self.cycler
            .cycle(&mut self.database, &self.configuration, messages)
    }
}
