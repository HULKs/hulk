use std::sync::Arc;

use color_eyre::Result;

use hsl_network_messages::{HulkMessage, PlayerNumber, StateMessage};
use ros_z::{prelude::*, qos::QosDurability};
use ros_z_streams::CreateAnnouncingPublisher;
use types::messages::IncomingMessage;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("message_filter").build().await?;

    let player_number_sub = node
        .subscriber::<PlayerNumber>("player_number")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let message_sub = node
        .subscriber::<IncomingMessage>("inputs/message")?
        .build()
        .await?;
    let filtered_message_pub = node
        .announcing_publisher::<IncomingMessage>("filtered_message")
        .await?;

    let mut player_number = None;

    loop {
        tokio::select! {
            message = message_sub.recv_with_metadata() => {
                let received_message = message?;

                let Some(current_player_number) = player_number else {continue;};
                let pending_accouncement = filtered_message_pub.announce(received_message.source_time).await?;
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
                    pending_accouncement.publish(&message).await?;
                }
            }
            received_player_number = player_number_sub.recv() => {
                player_number = Some(received_player_number?);
            }
        }
    }
}
