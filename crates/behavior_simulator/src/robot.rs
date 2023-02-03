use std::sync::Arc;

use color_eyre::Result;
use communication::server::Runtime;
use nalgebra::Isometry2;
use tokio_util::sync::CancellationToken;
use types::PrimaryState;

use crate::{
    cycler::{BehaviorCycler, Database},
    interfake::Interfake,
};

pub struct Robot {
    pub interface: Arc<Interfake>,
    pub cycler: BehaviorCycler<Interfake>,
    pub robot_to_field: Isometry2<f32>,
    pub chest_button_pressed: bool,

    pub database: Database,
    pub primary_state: PrimaryState,
}

impl Robot {
    pub fn new(index: usize) -> Self {
        let interface: Arc<_> = Interfake::default().into();
        let keep_running = CancellationToken::new();
        let communication_server = Runtime::<structs::Configuration>::start(
            None::<String>,
            "etc/configuration",
            format!("behavior_simulator{index}"),
            format!("behavior_simulator{index}"),
            2,
            keep_running.clone(),
        )
        .unwrap();

        let database_changed = std::sync::Arc::new(tokio::sync::Notify::new());
        let cycler = BehaviorCycler::new(
            interface.clone(),
            database_changed.clone(),
            communication_server.get_parameters_reader(),
        )
        .unwrap();

        keep_running.cancel();
        communication_server.join().unwrap().unwrap();

        Self {
            interface,
            cycler,
            robot_to_field: Isometry2::default(),
            chest_button_pressed: false,
            primary_state: PrimaryState::Unstiff,
            database: Database::default(),
        }
    }

    pub fn cycle(&mut self) -> Result<()> {
        self.database.main_outputs.buttons.is_chest_button_pressed = self.chest_button_pressed;
        self.database.main_outputs.primary_state = self.primary_state;
        self.database.main_outputs.robot_to_field = Some(self.robot_to_field);

        self.cycler.cycle(&mut self.database)
    }
}
