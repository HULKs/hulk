use std::{boxed::Box, future::Future, pin::Pin, sync::Arc};

use color_eyre::Result;
use hsl_network::endpoint::{Endpoint, Ports};
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::messages::OutgoingMessage;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
struct Parameters {
    ports: Ports,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("message_sender").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("message_receiver")?;
    let message_sub = node
        .subscriber::<OutgoingMessage>("outputs/message")?
        .build()
        .await?;

    let parameters = parameters.snapshot().typed().clone();
    let endpoint = Endpoint::new(parameters.ports).await?;

    loop {
        let message = message_sub.recv().await?;
        endpoint.write(message).await;
    }
}
