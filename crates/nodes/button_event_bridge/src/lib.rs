use std::sync::Arc;

use color_eyre::{
    Result,
    eyre::{Context as _, eyre},
};

use booster::ButtonEventMsg;
use ros_z::prelude::*;
use ros_z_streams::CreateAnnouncingPublisher;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("button_event_bridge").build().await?;

    let zenoh_session = ctx.session();

    let button_event_sub = zenoh_session
        .declare_subscriber("rt/button_event")
        .await
        .map_err(|error| color_eyre::eyre::eyre!("{error}"))?;
    let button_event_message_pub = node
        .announcing_publisher::<ButtonEventMsg>("inputs/button_event_message")
        .await?;

    loop {
        tokio::select! {
            button_event = button_event_sub.recv_async() => {
                let button_event = button_event.map_err(|error| eyre!("{error}"))?;
                let pending_accouncement = button_event_message_pub.announce(ctx.clock().now()).await?;

                let button_event = cdr::deserialize(&button_event.payload().to_bytes())
                    .wrap_err("deserialization failed")?;

                pending_accouncement
                    .publish(&button_event)
                    .await
                    ?;
            }
        }
    }
}
