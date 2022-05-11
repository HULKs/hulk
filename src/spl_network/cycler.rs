use std::{
    net::SocketAddr,
    sync::Arc,
    thread::{self, JoinHandle},
};

use anyhow::Result;
use log::error;
use spl_network::HULKS_TEAM_NUMBER;
use tokio::{net::UdpSocket, runtime, sync::Notify};
use tokio_util::sync::CancellationToken;

use crate::{
    control,
    framework::{
        buffer::{Reader, Writer},
        future_queue::Producer,
    },
    hardware::HardwareInterface,
    types::MessageEvent,
    CommunicationChannelsForCycler,
};

use super::{
    database::MainOutputs,
    game_controller_return_message_sender::send_game_controller_return_message,
    game_controller_state_message_parser::parse_game_controller_state_message,
    message_receiver::receive_message, spl_message_parser::parse_spl_message,
    spl_message_sender::spl_message_sender, Database,
};

#[allow(dead_code)]
pub struct SplNetwork<Hardware>
where
    Hardware: HardwareInterface + Sync + Send,
{
    hardware_interface: Arc<Hardware>,
    control_reader: Reader<control::Database>,
    spl_network_writer: Writer<Database>,
    spl_network_producer: Producer<MainOutputs>,
    communication_channels: CommunicationChannelsForCycler,
    control_database_changed: Arc<Notify>,

    last_game_controller_address: Option<SocketAddr>,
}

impl<Hardware> SplNetwork<Hardware>
where
    Hardware: HardwareInterface + Sync + Send + 'static,
{
    pub fn new(
        hardware_interface: Arc<Hardware>,
        control_reader: Reader<control::Database>,
        spl_network_writer: Writer<Database>,
        spl_network_producer: Producer<MainOutputs>,
        communication_channels: CommunicationChannelsForCycler,
        control_database_changed: Arc<Notify>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            hardware_interface,
            control_reader,
            spl_network_writer,
            spl_network_producer,
            communication_channels,
            control_database_changed,

            last_game_controller_address: Default::default(),
        })
    }

    pub fn start(mut self, keep_running: CancellationToken) -> JoinHandle<()> {
        thread::Builder::new()
            .name("spl_network".to_string())
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
                    let game_controller_state_messages = match UdpSocket::bind("0.0.0.0:3838").await
                    {
                        Ok(game_controller_messages) => game_controller_messages,
                        Err(error) => {
                            error!("Failed to bind GameController state socket: {:?}", error);
                            keep_running.cancel();
                            return;
                        }
                    };
                    let spl_messages = match UdpSocket::bind(format!(
                        "0.0.0.0:{}",
                        10000 + (HULKS_TEAM_NUMBER as u16)
                    ))
                    .await
                    {
                        Ok(spl_messages) => spl_messages,
                        Err(error) => {
                            error!("Failed to bind SPL message socket: {:?}", error);
                            keep_running.cancel();
                            return;
                        }
                    };
                    if let Err(error) = spl_messages.set_broadcast(true) {
                        error!(
                            "Failed to enable broadcast support for SPL message socket: {:?}",
                            error
                        );
                        keep_running.cancel();
                        return;
                    }
                    while !keep_running.is_cancelled() {
                        if let Err(error) = self
                            .cycle(
                                &keep_running,
                                &game_controller_state_messages,
                                &spl_messages,
                            )
                            .await
                        {
                            error!("`cycle` returned error: {:?}", error);
                            keep_running.cancel();
                        }
                    }
                });
            })
            .expect("Failed to spawn thread")
    }

    async fn cycle(
        &mut self,
        keep_running: &CancellationToken,
        game_controller_state_messages: &UdpSocket,
        spl_messages: &UdpSocket,
    ) -> Result<()> {
        {
            let mut spl_network_database = self.spl_network_writer.next();

            // reset
            spl_network_database
                .main_outputs
                .game_controller_state_message = None;
            spl_network_database.main_outputs.spl_message = None;

            // prepare
            let mut game_controller_state_message_buffer = [0; 1024];
            let mut spl_message_buffer = [0; 1024];
            let message_event = match receive_message(
                keep_running,
                self.control_reader.clone(),
                self.control_database_changed.clone(),
                &mut game_controller_state_message_buffer,
                &mut spl_message_buffer,
                game_controller_state_messages,
                spl_messages,
            )
            .await?
            {
                Some(message_event) => message_event,
                None => return Ok(()),
            };

            self.spl_network_producer.announce();

            // process
            match message_event {
                MessageEvent::GameControllerReturnMessageToBeSent { message } => {
                    send_game_controller_return_message(
                        game_controller_state_messages,
                        &self.last_game_controller_address,
                        message,
                    )
                    .await;
                }
                MessageEvent::SplMessageToBeSent { message } => {
                    spl_message_sender(spl_messages, message).await;
                }
                MessageEvent::IncomingGameControllerStateMessage { message, sender } => {
                    spl_network_database
                        .main_outputs
                        .game_controller_state_message =
                        parse_game_controller_state_message(message);
                    if spl_network_database
                        .main_outputs
                        .game_controller_state_message
                        .is_some()
                    {
                        self.last_game_controller_address = Some(sender);
                    }
                }
                MessageEvent::IncomingSplMessage { message, sender: _ } => {
                    spl_network_database.main_outputs.spl_message = parse_spl_message(message);
                }
            }

            self.spl_network_producer
                .finalize(spl_network_database.main_outputs.clone());
        }

        self.communication_channels.database_changed.notify_one();

        Ok(())
    }
}
