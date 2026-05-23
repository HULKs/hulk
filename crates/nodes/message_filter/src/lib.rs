use std::sync::Arc;

use color_eyre::Result;

use hsl_network_messages::{HulkMessage, PlayerNumber, StateMessage};
use ros_z::{prelude::*, qos::QosDurability};
use ros_z_streams::CreateAnnouncingPublisher;
use types::{messages::IncomingMessage, time_wrapper::TimeWrapper};

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
        .subscriber::<TimeWrapper<IncomingMessage>>("inputs/message")?
        .build()
        .await?;
    let filtered_message_pub = node
        .announcing_publisher::<IncomingMessage>("filtered_message")
        .await?;

    let mut player_number = None;

    loop {
        tokio::select! {
            received_time_wrapped_message = message_sub.recv() => {
                let Some(current_player_number) = player_number else {continue;};
                let time_wrapped_message = received_time_wrapped_message?;

                let pending_accouncement = filtered_message_pub.announce(time_wrapped_message.time).await?;

                let should_filter_message_out = matches!(time_wrapped_message.inner, IncomingMessage::Hsl(
                        HulkMessage::State(StateMessage { player_number, .. }),
                    ) if player_number == current_player_number);

                if !should_filter_message_out {
                    pending_accouncement.publish(&time_wrapped_message.inner).await?;
                }
            }
            received_player_number = player_number_sub.recv() => {
                player_number = Some(received_player_number?);
            }
        }
    }
}
