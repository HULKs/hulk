use std::sync::Arc;
use std::{boxed::Box, future::Future, pin::Pin};

use color_eyre::{
    Result,
    eyre::{Context as _, eyre},
};

use booster::ButtonEventMsg;
use ros_z::prelude::*;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("button_event_bridge").build().await?;

    let zenoh_session = ctx.session();

    let button_event_sub = zenoh_session
        .declare_subscriber("rt/button_event")
        .await
        .map_err(|error| color_eyre::eyre::eyre!("{error}"))?;
    let button_event_message_pub = node
        .publisher::<ButtonEventMsg>("inputs/button_event_message")?
        .build()
        .await?;

    loop {
        let button_event_message_sample = button_event_sub
            .recv_async()
            .await
            .map_err(|error| eyre!(error))?;

        let button_event_message =
            cdr::deserialize(&button_event_message_sample.payload().to_bytes())
                .wrap_err("deserialization failed")?;

        button_event_message_pub
            .publish(&button_event_message)
            .await?;
    }
}
