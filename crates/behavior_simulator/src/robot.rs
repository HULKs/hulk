use std::{ops::DerefMut, sync::Arc};

use color_eyre::Result;
use communication::server::Runtime;
use framework::Reader;
use nalgebra::Isometry2;
use tokio_util::sync::CancellationToken;

use crate::{
    cycler::{BehaviorCycler, Database},
    interfake::Interfake,
};

pub struct Robot {
    pub interface: Arc<Interfake>,
    pub cycler: BehaviorCycler<Interfake>,
    control_reader: Reader<Database>,
    pub robot_to_field: Isometry2<f32>,
    pub chest_button_pressed: bool,
    pub database_writer: framework::Writer<Database>,
}

impl Robot {
    pub fn new(index: usize, keep_running: CancellationToken) -> Self {
        let interface: Arc<_> = Interfake::default().into();
        let communication_server = Runtime::<structs::Configuration>::start(
            None::<String>,
            "etc/configuration",
            format!("behavior_simulator{index}"),
            format!("behavior_simulator{index}"),
            2,
            keep_running,
        )
        .unwrap();

        let (database_writer, database_reader) = framework::multiple_buffer_with_slots([
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        ]);

        let database_changed = std::sync::Arc::new(tokio::sync::Notify::new());
        let (subscribed_outputs_writer, _subscribed_outputs_reader) =
            framework::multiple_buffer_with_slots([
                Default::default(),
                Default::default(),
                Default::default(),
            ]);
        let cycler = BehaviorCycler::new(
            interface.clone(),
            database_changed.clone(),
            communication_server.get_parameters_reader(),
        )
        .unwrap();
        communication_server.register_cycler_instance(
            "Control",
            database_changed,
            database_reader.clone(),
            subscribed_outputs_writer,
        );

        Self {
            interface,
            cycler,
            control_reader: database_reader,
            robot_to_field: Isometry2::default(),
            chest_button_pressed: false,
            database_writer,
        }
    }

    pub fn cycle(&mut self) -> Result<()> {
        let mut own_database = self.database_writer.next();
        let own_database_reference = own_database.deref_mut();

        own_database_reference
            .main_outputs
            .buttons
            .is_chest_button_pressed = self.chest_button_pressed;
        own_database_reference.main_outputs.robot_to_field = Some(self.robot_to_field);

        self.cycler.cycle(own_database_reference)
    }

    pub fn get_database(&self) -> Database {
        let database = self.control_reader.next();
        database.clone()
    }
}
