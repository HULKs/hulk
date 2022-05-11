use std::{
    collections::HashSet,
    sync::Arc,
    thread::{self, JoinHandle},
};

use log::error;
use tokio::{
    runtime,
    sync::{broadcast::Sender, mpsc::channel, Notify},
};
use tokio_util::sync::CancellationToken;

use crate::{
    audio, control,
    framework::{
        buffer::{Reader, Writer},
        communication::{
            acceptor::acceptor, database_subscription_manager::database_subscription_manager,
        },
        Configuration,
    },
    spl_network, vision, CommunicationChannelsForCommunication,
    CommunicationChannelsForCommunicationWithImage,
};

use super::parameter_modificator::parameter_modificator;

pub struct ChannelsForDatabases<Database> {
    pub database: Reader<Database>,
    pub database_changed: Arc<Notify>,
    pub subscribed_additional_outputs: Writer<HashSet<String>>,
}

pub struct ChannelsForDatabasesWithImage<Database> {
    pub database: Reader<Database>,
    pub database_changed: Arc<Notify>,
    pub subscribed_additional_outputs: Writer<HashSet<String>>,
    pub subscribed_image: Writer<bool>,
}

pub struct ChannelsForParameters {
    pub configuration: Writer<Configuration>,
    pub changed_parameters: Sender<String>,
}

pub struct Communication {
    configuration: Writer<Configuration>,
    initial_configuration: Configuration,
    changed_parameters: Sender<String>,
    channels_from_audio: CommunicationChannelsForCommunication<audio::Database>,
    channels_from_control: CommunicationChannelsForCommunication<control::Database>,
    channels_from_spl_network: CommunicationChannelsForCommunication<spl_network::Database>,
    channels_from_vision_top: CommunicationChannelsForCommunicationWithImage<vision::Database>,
    channels_from_vision_bottom: CommunicationChannelsForCommunicationWithImage<vision::Database>,
}

impl Communication {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        configuration: Writer<Configuration>,
        initial_configuration: Configuration,
        changed_parameters: Sender<String>,
        channels_from_audio: CommunicationChannelsForCommunication<audio::Database>,
        channels_from_control: CommunicationChannelsForCommunication<control::Database>,
        channels_from_spl_network: CommunicationChannelsForCommunication<spl_network::Database>,
        channels_from_vision_top: CommunicationChannelsForCommunicationWithImage<vision::Database>,
        channels_from_vision_bottom: CommunicationChannelsForCommunicationWithImage<
            vision::Database,
        >,
    ) -> Self {
        Self {
            configuration,
            initial_configuration,
            changed_parameters,
            channels_from_audio,
            channels_from_control,
            channels_from_spl_network,
            channels_from_vision_top,
            channels_from_vision_bottom,
        }
    }

    pub fn start(self, keep_running: CancellationToken) -> JoinHandle<()> {
        thread::Builder::new()
            .name("communication".to_string())
            .spawn(move || {
                let runtime = match runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        error!("Failed to build runtime: {:?}", error);
                        keep_running.cancel();
                        return;
                    }
                };
                runtime.block_on(async move {
                    let (
                        database_subscription_manager_sender,
                        database_subscription_manager_receiver,
                    ) = channel(1);
                    let (parameter_modificator_sender, parameter_modificator_receiver) = channel(1);
                    let channels_for_audio_databases = ChannelsForDatabases {
                        database: self.channels_from_audio.database,
                        database_changed: self.channels_from_audio.database_changed,
                        subscribed_additional_outputs: self
                            .channels_from_audio
                            .subscribed_additional_outputs,
                    };
                    let channels_for_control_databases = ChannelsForDatabases {
                        database: self.channels_from_control.database,
                        database_changed: self.channels_from_control.database_changed,
                        subscribed_additional_outputs: self
                            .channels_from_control
                            .subscribed_additional_outputs,
                    };
                    let channels_for_spl_network_databases = ChannelsForDatabases {
                        database: self.channels_from_spl_network.database,
                        database_changed: self.channels_from_spl_network.database_changed,
                        subscribed_additional_outputs: self
                            .channels_from_spl_network
                            .subscribed_additional_outputs,
                    };
                    let channels_for_vision_top_databases = ChannelsForDatabasesWithImage {
                        database: self.channels_from_vision_top.database,
                        database_changed: self.channels_from_vision_top.database_changed,
                        subscribed_additional_outputs: self
                            .channels_from_vision_top
                            .subscribed_additional_outputs,
                        subscribed_image: self.channels_from_vision_top.subscribed_image,
                    };
                    let channels_for_vision_bottom_databases = ChannelsForDatabasesWithImage {
                        database: self.channels_from_vision_bottom.database,
                        database_changed: self.channels_from_vision_bottom.database_changed,
                        subscribed_additional_outputs: self
                            .channels_from_vision_bottom
                            .subscribed_additional_outputs,
                        subscribed_image: self.channels_from_vision_bottom.subscribed_image,
                    };
                    let channels_for_parameters = ChannelsForParameters {
                        configuration: self.configuration,
                        changed_parameters: self.changed_parameters,
                    };

                    let database_subscription_manager_task = database_subscription_manager(
                        database_subscription_manager_receiver,
                        channels_for_audio_databases,
                        channels_for_control_databases,
                        channels_for_spl_network_databases,
                        channels_for_vision_top_databases,
                        channels_for_vision_bottom_databases,
                    )
                    .await;
                    let parameter_modificator_task = parameter_modificator(
                        parameter_modificator_receiver,
                        self.initial_configuration,
                        channels_for_parameters,
                    )
                    .await;
                    let acceptor_task = acceptor(
                        database_subscription_manager_sender,
                        parameter_modificator_sender,
                        keep_running.clone(),
                    )
                    .await;
                    keep_running.cancelled().await;
                    let acceptor_task_result = acceptor_task.await;
                    let database_subscription_manager_task_result =
                        database_subscription_manager_task.await;
                    let parameter_modificator_task_result = parameter_modificator_task.await;
                    if let Err(error) = acceptor_task_result {
                        error!("Got error during `acceptor`: {:?}", error);
                    }
                    if let Err(error) = database_subscription_manager_task_result {
                        error!(
                            "Got error during `database_subscription_manager`: {:?}",
                            error
                        );
                    }
                    if let Err(error) = parameter_modificator_task_result {
                        error!("Got error during `parameter_modificator`: {:?}", error);
                    }
                });
            })
            .expect("Failed to spawn thread")
    }
}
