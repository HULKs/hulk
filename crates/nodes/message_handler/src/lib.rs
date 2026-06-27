use std::{boxed::Box, future::Future, pin::Pin, sync::Arc};

use color_eyre::Result;
use hsl_network::endpoint::{Endpoint, Ports};
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{
    messages::{IncomingMessage, OutgoingMessage},
    time_wrapper::TimeWrapper,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
struct Parameters {
    ports: Ports,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("message_handler").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("message_receiver")?;
    let incoming_message_pub = node
        .publisher::<TimeWrapper<IncomingMessage>>("inputs/message")
        .build()
        .await?;
    let outgoing_message_sub = node
        .subscriber::<OutgoingMessage>("outputs/message")
        .build()
        .await?;

    let parameters = parameters.snapshot().typed().clone();
    let endpoint = Endpoint::new(parameters.ports).await?;

    loop {
        tokio::select! {
            message = endpoint.read() => {
                let message = TimeWrapper {
                    time: ctx.clock().now(),
                    inner: message?,
                };
                incoming_message_pub.publish(&message).await?;
            }
            message = outgoing_message_sub.recv() => {
                endpoint.write(message?).await;
            }
        }
    }
}
