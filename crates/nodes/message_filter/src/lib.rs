use std::sync::Arc;

use color_eyre::Result;

use hsl_network_messages::{HulkMessage, PlayerNumber, StateMessage};
use ros_z::{prelude::*, qos::QosDurability};
use ros_z_streams::CreateAnnouncingPublisher;
use types::messages::{IncomingMessage, StampedIncomingMessage};

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
        .subscriber::<StampedIncomingMessage>("inputs/message")?
        .build()
        .await?;
    let filtered_message_pub = node
        .announcing_publisher::<StampedIncomingMessage>("filtered_message")
        .await?;

    let mut player_number = None;

    loop {
        tokio::select! {
            received_stamped_message = message_sub.recv() => {
                let Some(current_player_number) = player_number else {continue;};
                let stamped_message = received_stamped_message?;

                let pending_accouncement = filtered_message_pub.announce(stamped_message.time).await?;

                let should_filter_message_out = matches!(stamped_message.incoming_message, IncomingMessage::Hsl(
                        message @ HulkMessage::State(StateMessage { player_number, .. }),
                    ) if player_number == current_player_number);

                if !should_filter_message_out {
                    pending_accouncement.publish(&stamped_message).await?;
                }
            }
            received_player_number = player_number_sub.recv() => {
                player_number = Some(received_player_number?);
            }
        }
    }
}
