use std::sync::Arc;

use color_eyre::Result;

use hsl_network_messages::{HulkMessage, PlayerNumber, StateMessage};
use ros_z::{prelude::*, qos::QosDurability};
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
        .publisher::<IncomingMessage>("filtered_message")?
        .build()
        .await?;

    let mut player_number = None;

    loop {
        tokio::select! {
            message = message_sub.recv(), if player_number.is_some() => {
                let message = match message.into_eyre()? {
                    IncomingMessage::GameController(source_address, message) => Some(
                        IncomingMessage::GameController(source_address, message.clone()),
                    ),
                    IncomingMessage::Hsl(
                        message @ HulkMessage::State(StateMessage { player_number, .. }),
                    ) if player_number != player_number => Some(IncomingMessage::Hsl(message)),
                    _ => None,
                };

                if let Some(message) = message {
                    filtered_message_pub.publish(&message).await.into_eyre()?;
                }
            }
            new_player_number = player_number_sub.recv() => {
                player_number = Some(new_player_number);
            }
        }
    }
}
