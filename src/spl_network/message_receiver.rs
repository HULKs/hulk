use std::sync::Arc;

use anyhow::bail;
use tokio::{net::UdpSocket, select, sync::Notify};
use tokio_util::sync::CancellationToken;

use crate::{control::Database, framework::buffer::Reader, types::MessageEvent};

pub async fn receive_message<'buffer>(
    keep_running: &CancellationToken,
    control_reader: Reader<Database>,
    control_database_changed: Arc<Notify>,
    game_controller_state_message_buffer: &'buffer mut [u8],
    spl_message_buffer: &'buffer mut [u8],
    incoming_game_controller_state_messages: &UdpSocket,
    incoming_spl_messages: &UdpSocket,
) -> anyhow::Result<Option<MessageEvent<'buffer>>> {
    select! {
        _ = keep_running.cancelled() => Ok(None),
        message_event = async move {
            loop {
                let control_database = control_reader.next();
                let message_receivers = match &control_database.main_outputs.message_receivers {
                    Some(message_receivers) => message_receivers,
                    None => {
                        control_database_changed.notified().await;
                        continue;
                    },
                };

                let mut game_controller_return_message_receiver = message_receivers
                    .game_controller_return_message_receiver
                    .lock()
                    .await;
                let mut spl_message_receiver = message_receivers
                    .spl_message_receiver
                    .lock()
                    .await;

                select! {
                    message = game_controller_return_message_receiver.recv() => {
                        match message {
                            Some(message) => return Ok(Some(MessageEvent::GameControllerReturnMessageToBeSent{
                                message,
                            })),
                            None => bail!("Failed to receive from GameController return message receiver"),
                        }
                    },
                    message = spl_message_receiver.recv() => {
                        match message {
                            Some(message) => return Ok(Some(MessageEvent::SplMessageToBeSent{
                                message,
                            })),
                            None => bail!("Failed to receive from SPL message receiver"),
                        }
                    },
                    message_result = incoming_game_controller_state_messages.recv_from(
                        game_controller_state_message_buffer
                    ) => {
                        match message_result {
                            Ok((number_of_bytes, sender)) => return Ok(Some(MessageEvent::IncomingGameControllerStateMessage{
                                message: &game_controller_state_message_buffer[..number_of_bytes],
                                sender,
                            })),
                            Err(error) => bail!("Failed to receive from GameController state socket: {:?}", error),
                        }
                    },
                    message_result = incoming_spl_messages.recv_from(
                        spl_message_buffer
                    ) => {
                        match message_result {
                            Ok((number_of_bytes, sender)) => return Ok(Some(MessageEvent::IncomingSplMessage{
                                message: &spl_message_buffer[..number_of_bytes],
                                sender,
                            })),
                            Err(error) => bail!("Failed to receive from SPL message socket: {:?}", error),
                        }
                    },
                }
            }
        } => message_event
    }
}
