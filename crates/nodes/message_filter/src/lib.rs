use std::{future::pending, sync::Arc};

use color_eyre::Result;

use hsl_network_messages::PlayerNumber;
use ros_z::{prelude::*, qos::QosDurability};
use types::messages::IncomingMessage;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("message_filter").build().await?;

    let _player_number_sub = node
        .subscriber::<PlayerNumber>("player_number")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let _message_sub = node
        .subscriber::<IncomingMessage>("inputs/message")?
        .build()
        .await?;
    let _filtered_message_pub = node
        .publisher::<IncomingMessage>("filtered_message")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
