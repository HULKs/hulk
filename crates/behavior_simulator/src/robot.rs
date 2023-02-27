use std::{collections::BTreeMap, sync::Arc, time::SystemTime};

use color_eyre::Result;
use communication::server::Runtime;
use nalgebra::{Isometry2, Translation2, UnitComplex};
use structs::Configuration;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
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
    pub fn new(player_number: usize) -> Self {
        let interface: Arc<_> = Interfake::default().into();
        let keep_running = CancellationToken::new();
        let communication_server = Runtime::<Configuration>::start(
            None::<String>,
            "etc/configuration",
            format!("behavior_simulator{player_number}"),
            format!("behavior_simulator{player_number}"),
            2,
            keep_running.clone(),
        )
        .unwrap();

        let mut configuration = communication_server.get_parameters_reader().next().clone();
        configuration.player_number = player_number.try_into().unwrap();

        let database_changed = Arc::new(Notify::new());
        let cycler =
            BehaviorCycler::new(interface.clone(), database_changed, &configuration).unwrap();

        keep_running.cancel();
        communication_server.join().unwrap().unwrap();

        let mut database = Database::default();

        let (y, x) = (player_number as f32).sin_cos();
        let position = Translation2::new(x * 2.0, y * 2.0);
        database.main_outputs.robot_to_field = Some(Isometry2::from_parts(
            position,
            UnitComplex::from_angle(0.0),
        ));

        Self {
            interface,
            cycler,

            database,
            configuration,

            penalized: false,
        }
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
