use std::{
    collections::BTreeMap,
    convert::Into,
    sync::Arc,
    time::{Duration, SystemTime},
};

use color_eyre::{eyre::WrapErr, Result};
use control::localization::generate_initial_pose;
use nalgebra::vector;
use parameters::directory::deserialize;
use spl_network_messages::PlayerNumber;
use types::{messages::IncomingMessage, CameraMatrix};

use crate::{
    cycler::{BehaviorCycler, Database},
    interfake::Interfake,
    structs::Configuration,
};

pub struct Robot {
    pub interface: Arc<Interfake>,
    pub cycler: BehaviorCycler,
    pub database: Database,
    pub configuration: Configuration,
    pub is_penalized: bool,
    pub last_kick_time: Duration,
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
                &format!("behavior_simulator.{}", from_player_number(player_number)),
                &format!("behavior_simulator.{}", from_player_number(player_number)),
            )
            .await
            .wrap_err("could not load initial parameters")
        })?;
        configuration.player_number = player_number;

        let cycler = BehaviorCycler::new(interface.clone(), Default::default(), &configuration)
            .wrap_err("failed to create cycler")?;

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
            is_penalized: false,
            last_kick_time: Duration::default(),
        })
    }

    pub fn cycle(&mut self, messages: BTreeMap<SystemTime, Vec<&IncomingMessage>>) -> Result<()> {
        self.cycler
            .cycle(&mut self.database, &self.configuration, messages)
    }

    pub fn field_of_view(&self) -> f32 {
        let image_size = vector![640.0, 480.0];
        let focal_lengths = self
            .configuration
            .camera_matrix_parameters
            .vision_top
            .focal_lengths;
        let focal_lengths_scaled = image_size.component_mul(&focal_lengths);
        let field_of_view = CameraMatrix::calculate_field_of_view(focal_lengths_scaled, image_size);

        field_of_view.x
    }
}

pub fn to_player_number(value: usize) -> Result<PlayerNumber, String> {
    let number = match value {
        1 => PlayerNumber::One,
        2 => PlayerNumber::Two,
        3 => PlayerNumber::Three,
        4 => PlayerNumber::Four,
        5 => PlayerNumber::Five,
        6 => PlayerNumber::Six,
        7 => PlayerNumber::Seven,
        number => return Err(format!("invalid player number: {number}")),
    };

    Ok(number)
}

pub fn from_player_number(val: PlayerNumber) -> usize {
    match val {
        PlayerNumber::One => 1,
        PlayerNumber::Two => 2,
        PlayerNumber::Three => 3,
        PlayerNumber::Four => 4,
        PlayerNumber::Five => 5,
        PlayerNumber::Six => 6,
        PlayerNumber::Seven => 7,
    }
}
