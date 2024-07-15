use std::{
    convert::Into,
    sync::{mpsc, Arc},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use buffered_watch::Receiver;
use color_eyre::{eyre::WrapErr, Result};

use control::localization::generate_initial_pose;
use framework::{future_queue, Producer, RecordingTrigger};
use linear_algebra::vector;
use parameters::directory::deserialize;
use projection::camera_matrix::CameraMatrix;
use spl_network_messages::{HulkMessage, PlayerNumber};
use types::{hardware::Ids, messages::IncomingMessage, motion_selection::MotionSafeExits};

use crate::{
    cyclers::control::{Cycler, CyclerInstance, Database},
    interfake::{FakeDataInterface, Interfake},
    structs::Parameters,
};

pub struct Robot {
    pub interface: Arc<Interfake>,
    pub database: Database,
    pub parameters: Parameters,
    pub is_penalized: bool,
    pub last_kick_time: Duration,
    pub ball_last_seen: Option<SystemTime>,

    cycler: Cycler<Interfake>,
    control_receiver: Receiver<(SystemTime, Database)>,
    spl_network_sender: Producer<crate::structs::spl_network::MainOutputs>,
}

impl Robot {
    pub fn try_new(player_number: PlayerNumber) -> Result<Self> {
        let ids = Ids {
            body_id: format!("behavior_simulator.{}", from_player_number(player_number)),
            head_id: format!("behavior_simulator.{}", from_player_number(player_number)),
        };
        let mut parameter: Parameters =
            deserialize("etc/parameters", &ids).wrap_err("could not load initial parameters")?;
        parameter.player_number = player_number;

        let interface: Arc<_> = Interfake::default().into();

        let (control_sender, control_receiver) =
            buffered_watch::channel((UNIX_EPOCH, Database::default()));
        let (mut subscriptions_sender, subscriptions_receiver) =
            buffered_watch::channel(Default::default());
        let (mut parameters_sender, parameters_receiver) =
            buffered_watch::channel((UNIX_EPOCH, Default::default()));
        let (spl_network_sender, spl_network_consumer) = future_queue();
        let (recording_sender, _recording_receiver) = mpsc::sync_channel(0);
        *parameters_sender.borrow_mut() = (SystemTime::now(), parameter.clone());

        let mut cycler = Cycler::new(
            CyclerInstance::Control,
            interface.clone(),
            control_sender,
            subscriptions_receiver,
            parameters_receiver,
            spl_network_consumer,
            recording_sender,
            RecordingTrigger::new(0),
        )?;
        cycler.cycler_state.motion_safe_exits = MotionSafeExits::fill(true);

        let mut database = Database::default();

        database.main_outputs.ground_to_field = Some(
            generate_initial_pose(
                &parameter.localization.initial_poses[player_number],
                &parameter.field_dimensions,
            )
            .as_transform(),
        );
        database.main_outputs.has_ground_contact = true;
        database.main_outputs.is_localization_converged = true;
        subscriptions_sender
            .borrow_mut()
            .insert("additional_outputs".to_string());

        Ok(Self {
            interface,
            database,
            parameters: parameter,
            is_penalized: false,
            last_kick_time: Duration::default(),
            ball_last_seen: None,

            cycler,
            control_receiver,
            spl_network_sender,
        })
    }

    pub fn cycle(&mut self, messages: &[(PlayerNumber, HulkMessage)]) -> Result<()> {
        for (source, hulks_message) in messages.iter() {
            let source_is_other = *source != self.parameters.player_number;
            let message = IncomingMessage::Spl(*hulks_message);
            self.spl_network_sender.announce();
            self.spl_network_sender
                .finalize(crate::structs::spl_network::MainOutputs {
                    filtered_message: source_is_other.then(|| message.clone()),
                    message,
                });
        }
        buffered_watch::Sender::<_>::borrow_mut(
            &mut self.interface.get_last_database_sender().lock(),
        )
        .main_outputs = self.database.main_outputs.clone();

        self.cycler.cycle()?;

        let (_, database) = &*self.control_receiver.borrow_and_mark_as_seen();
        self.database.main_outputs = database.main_outputs.clone();
        self.database.additional_outputs = database.additional_outputs.clone();
        Ok(())
    }

    pub fn field_of_view(&self) -> f32 {
        let image_size = vector![640.0, 480.0];
        let focal_lengths = self
            .parameters
            .camera_matrix_parameters
            .vision_top
            .focal_lengths;
        let focal_lengths_scaled = image_size.inner.cast().component_mul(&focal_lengths);
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
