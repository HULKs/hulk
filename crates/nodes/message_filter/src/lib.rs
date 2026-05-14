use std::{future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use hsl_network_messages::PlayerNumber;
use ros_z::{IntoEyreResultExt, prelude::*};
use types::messages::IncomingMessage;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub player_number: PlayerNumber,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("message_filter")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("message_filter")
        .into_eyre()?;
    let _message_sub = node
        .subscriber::<IncomingMessage>("inputs/message")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _filtered_message_pub = node
        .publisher::<IncomingMessage>("filtered_message")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
