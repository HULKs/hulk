use std::sync::Arc;

use color_eyre::Result;

use hsl_network_messages::{HulkMessage, PlayerNumber, StateMessage};
use ros_z::{IntoEyreResultExt, prelude::*, qos::QosDurability};
use ros_z_streams::CreateAnnouncingPublisher;
use types::messages::IncomingMessage;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("message_filter")
        .build()
        .await
        .into_eyre()?;

    let player_number_sub = node
        .subscriber::<PlayerNumber>("player_number")
        .into_eyre()?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await
        .into_eyre()?;
    let message_sub = node
        .subscriber::<IncomingMessage>("inputs/message")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let filtered_message_pub = node
        .announcing_publisher::<IncomingMessage>("filtered_message")
        .await
        .into_eyre()?;

    let mut player_number = None;

    loop {
        tokio::select! {
            message = message_sub.recv_with_metadata() => {
                let received_message = message.into_eyre()?;

                let Some(current_player_number) = player_number else {continue;};
                let pending_accouncement = filtered_message_pub.announce(received_message.source_time).await.into_eyre()?;
                let message = match received_message.into_message(){
                    IncomingMessage::GameController(source_address, message) => Some(
                        IncomingMessage::GameController(source_address, message.clone()),
                    ),
                    IncomingMessage::Hsl(
                        message @ HulkMessage::State(StateMessage { player_number, .. }),
                    ) if player_number != current_player_number => Some(IncomingMessage::Hsl(message)),
                    _ => None,
                };

                if let Some(message) = message {
                    pending_accouncement.publish(&message).await.into_eyre()?;
                }
            }
            received_player_number = player_number_sub.recv() => {
                player_number = Some(received_player_number.into_eyre()?);
            }
        }
    }
}
