use std::sync::Arc;

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use hsl_network::endpoint::{Endpoint, Ports};
use ros_z::prelude::*;
use types::{messages::IncomingMessage, time_wrapper::TimeWrapper};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
struct Parameters {
    ports: Ports,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("message_receiver").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("message_receiver")?;
    let message_pub = node
        .publisher::<TimeWrapper<IncomingMessage>>("inputs/message")?
        .build()
        .await?;

    let parameters = parameters.snapshot().typed().clone();
    let endpoint = Endpoint::new(parameters.ports).await?;

    loop {
        tokio::select! {
            message = endpoint.read() => {
                let message = message?;

                let message = TimeWrapper{ time: ctx.clock().now(), inner: message };
                message_pub.publish(&message).await?;
            }
        }
    }
}
