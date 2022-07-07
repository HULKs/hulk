use std::{collections::HashSet, panic, sync::Arc, thread::JoinHandle};

use anyhow::Context;
use serde_json::from_value;
use tokio::sync::{
    broadcast::{channel, Receiver},
    Notify,
};
use tokio_util::sync::CancellationToken;
use types::CameraPosition;

use crate::{
    audio::Audio,
    control::Control,
    framework::{
        buffer::{self, Reader, Writer},
        communication::{configuration_directory::deserialize, Communication},
        future_queue, Configuration,
    },
    hardware::HardwareInterface,
    spl_network::SplNetwork,
    vision::Vision,
};

pub struct CommunicationChannelsForCycler {
    pub database_changed: Arc<Notify>,
    pub subscribed_additional_outputs: Reader<HashSet<String>>,
    pub configuration: Reader<Configuration>,
    pub changed_parameters: Receiver<String>,
}

pub struct CommunicationChannelsForCyclerWithImage {
    pub database_changed: Arc<Notify>,
    pub subscribed_additional_outputs: Reader<HashSet<String>>,
    pub subscribed_image: Reader<bool>,
    pub configuration: Reader<Configuration>,
    pub changed_parameters: Receiver<String>,
}

pub struct CommunicationChannelsForCommunication<Database> {
    pub database: Reader<Database>,
    pub database_changed: Arc<Notify>,
    pub subscribed_additional_outputs: Writer<HashSet<String>>,
}

pub struct CommunicationChannelsForCommunicationWithImage<Database> {
    pub database: Reader<Database>,
    pub database_changed: Arc<Notify>,
    pub subscribed_additional_outputs: Writer<HashSet<String>>,
    pub subscribed_image: Writer<bool>,
}

fn new_communication_channels<Database>(
    database: Reader<Database>,
    configuration: Reader<Configuration>,
    changed_parameters: Receiver<String>,
) -> (
    CommunicationChannelsForCycler,
    CommunicationChannelsForCommunication<Database>,
) {
    let database_changed = Arc::new(Notify::new());
    let (subscribed_additional_outputs_writer, subscribed_additional_outputs_reader) =
        buffer::with_slots([Default::default(), Default::default(), Default::default()]);

    (
        CommunicationChannelsForCycler {
            database_changed: database_changed.clone(),
            subscribed_additional_outputs: subscribed_additional_outputs_reader,
            configuration,
            changed_parameters,
        },
        CommunicationChannelsForCommunication {
            database,
            database_changed,
            subscribed_additional_outputs: subscribed_additional_outputs_writer,
        },
    )
}

fn new_communication_channels_with_image<Database>(
    database: Reader<Database>,
    configuration: Reader<Configuration>,
    changed_parameters: Receiver<String>,
) -> (
    CommunicationChannelsForCyclerWithImage,
    CommunicationChannelsForCommunicationWithImage<Database>,
) {
    let database_changed = Arc::new(Notify::new());
    let (subscribed_additional_outputs_writer, subscribed_additional_outputs_reader) =
        buffer::with_slots([Default::default(), Default::default(), Default::default()]);
    let (subscribed_image_writer, subscribed_image_reader) =
        buffer::with_slots([Default::default(), Default::default(), Default::default()]);

    (
        CommunicationChannelsForCyclerWithImage {
            database_changed: database_changed.clone(),
            subscribed_additional_outputs: subscribed_additional_outputs_reader,
            subscribed_image: subscribed_image_reader,
            configuration,
            changed_parameters,
        },
        CommunicationChannelsForCommunicationWithImage {
            database,
            database_changed,
            subscribed_additional_outputs: subscribed_additional_outputs_writer,
            subscribed_image: subscribed_image_writer,
        },
    )
}

pub struct Runtime<Hardware>
where
    Hardware: HardwareInterface + Sync + Send,
{
    audio: Audio<Hardware>,
    control: Control<Hardware>,
    spl_network: SplNetwork<Hardware>,
    vision_top: Vision<Hardware>,
    vision_bottom: Vision<Hardware>,
    communication: Communication,
}

impl<Hardware> Runtime<Hardware>
where
    Hardware: HardwareInterface + Sync + Send + 'static,
{
    pub fn construct(hardware_interface: Arc<Hardware>) -> anyhow::Result<Self> {
        let initial_configuration: Configuration = from_value(deserialize(
            "etc/configuration",
            hardware_interface.get_ids(),
        )?)
        .context("Failed to read configuration")?;

        let (configuration_writer, configuration_reader) = buffer::with_slots([
            initial_configuration.clone(),
            initial_configuration.clone(),
            initial_configuration.clone(),
            initial_configuration.clone(),
            initial_configuration.clone(),
            initial_configuration.clone(),
            initial_configuration.clone(),
        ]);
        let (changed_parameters_sender, changed_parameters_receiver) = channel(42);

        let (audio_database_writer, audio_database_reader) = buffer::with_slots([
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        ]);
        let (control_database_writer, control_database_reader) = buffer::with_slots([
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        ]);
        let (spl_network_database_writer, spl_network_database_reader) = buffer::with_slots([
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        ]);
        let (vision_top_database_writer, vision_top_database_reader) = buffer::with_slots([
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        ]);
        let (vision_bottom_database_writer, vision_bottom_database_reader) = buffer::with_slots([
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        ]);

        let (channels_for_audio, channels_from_audio) = new_communication_channels(
            audio_database_reader,
            configuration_reader.clone(),
            changed_parameters_receiver,
        );
        let changed_parameters_receiver = changed_parameters_sender.subscribe();
        let (channels_for_control, channels_from_control) = new_communication_channels(
            control_database_reader.clone(),
            configuration_reader.clone(),
            changed_parameters_receiver,
        );
        let control_database_changed = channels_from_control.database_changed.clone();
        let changed_parameters_receiver = changed_parameters_sender.subscribe();
        let (channels_for_spl_network, channels_from_spl_network) = new_communication_channels(
            spl_network_database_reader,
            configuration_reader.clone(),
            changed_parameters_receiver,
        );
        let changed_parameters_receiver = changed_parameters_sender.subscribe();
        let (channels_for_vision_top, channels_from_vision_top) =
            new_communication_channels_with_image(
                vision_top_database_reader,
                configuration_reader.clone(),
                changed_parameters_receiver,
            );
        let changed_parameters_receiver = changed_parameters_sender.subscribe();
        let (channels_for_vision_bottom, channels_from_vision_bottom) =
            new_communication_channels_with_image(
                vision_bottom_database_reader,
                configuration_reader,
                changed_parameters_receiver,
            );

        let (audio_database_producer, audio_database_consumer) = future_queue::new();
        let (spl_network_database_producer, spl_network_database_consumer) = future_queue::new();
        let (vision_top_database_producer, vision_top_database_consumer) = future_queue::new();
        let (vision_bottom_database_producer, vision_bottom_database_consumer) =
            future_queue::new();

        let audio = Audio::new(
            hardware_interface.clone(),
            control_database_reader.clone(),
            audio_database_writer,
            audio_database_producer,
            channels_for_audio,
        )
        .context("Failed to construct audio cycler")?;
        let control = Control::new(
            hardware_interface.clone(),
            control_database_writer,
            spl_network_database_consumer,
            vision_top_database_consumer,
            vision_bottom_database_consumer,
            audio_database_consumer,
            channels_for_control,
        )
        .context("Failed to construct control cycler")?;
        let spl_network = SplNetwork::new(
            hardware_interface.clone(),
            control_database_reader.clone(),
            spl_network_database_writer,
            spl_network_database_producer,
            channels_for_spl_network,
            control_database_changed,
        )
        .context("Failed to construct spl_network cycler")?;
        let vision_top = Vision::new(
            CameraPosition::Top,
            hardware_interface.clone(),
            control_database_reader.clone(),
            vision_top_database_writer,
            vision_top_database_producer,
            channels_for_vision_top,
        )
        .context("Failed to construct vision_top cycler")?;
        let vision_bottom = Vision::new(
            CameraPosition::Bottom,
            hardware_interface,
            control_database_reader,
            vision_bottom_database_writer,
            vision_bottom_database_producer,
            channels_for_vision_bottom,
        )
        .context("Failed to construct vision_bottom cycler")?;

        let communication = Communication::new(
            configuration_writer,
            initial_configuration,
            changed_parameters_sender,
            channels_from_audio,
            channels_from_control,
            channels_from_spl_network,
            channels_from_vision_top,
            channels_from_vision_bottom,
        );

        Ok(Self {
            audio,
            control,
            spl_network,
            vision_top,
            vision_bottom,
            communication,
        })
    }

    pub fn run(self, keep_running: CancellationToken) -> anyhow::Result<()> {
        let audio = self.audio.start(keep_running.clone());
        let control = self.control.start(keep_running.clone());
        let spl_network = self.spl_network.start(keep_running.clone());
        let vision_top = self.vision_top.start(keep_running.clone());
        let vision_bottom = self.vision_bottom.start(keep_running.clone());
        let communication = self.communication.start(keep_running);

        panic_join(audio);
        panic_join(control);
        panic_join(spl_network);
        panic_join(vision_top);
        panic_join(vision_bottom);
        panic_join(communication);
        Ok(())
    }
}

fn panic_join(handle: JoinHandle<()>) {
    if let Err(error) = handle.join() {
        panic::resume_unwind(error)
    }
}
