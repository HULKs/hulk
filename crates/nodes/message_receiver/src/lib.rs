use std::sync::Arc;

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use hsl_network::endpoint::{Endpoint, Ports};
use ros_z::{IntoEyreResultExt, prelude::*};
use types::messages::IncomingMessage;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
struct Parameters {
    ports: Ports,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("message_receiver")
        .build()
        .await
        .into_eyre()?;

    let parameters = node
        .bind_parameter_as::<Parameters>("message_receiver")
        .into_eyre()?;
    let message_pub = node
        .publisher::<IncomingMessage>("inputs/message")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    let parameters = parameters.snapshot().typed().clone();
    let endpoint = Endpoint::new(parameters.ports).await.into_eyre()?;

    loop {
        tokio::select! {
            message = endpoint.read() => {
                let message = message.into_eyre()?;

                message_pub.publish(&message).await.into_eyre()?;
            }
        }
    }
}
