use std::{future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use hsl_network_messages::PlayerNumber;
use ros_z::prelude::*;
use types::messages::IncomingMessage;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub player_number: PlayerNumber,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("message_filter").build().await?;

    let _parameters = node.bind_parameter_as::<Parameters>("message_filter")?;
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
