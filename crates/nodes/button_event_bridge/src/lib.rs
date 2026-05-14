use std::sync::Arc;

use color_eyre::{Result, eyre::Context as _};

use booster::ButtonEventMsg;
use ros_z::{IntoEyreResultExt, prelude::*};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("button_event_bridge")
        .build()
        .await
        .into_eyre()?;

    let zenoh_session = ctx.session();

    let button_event_sub = zenoh_session
        .declare_subscriber("rt/button_event")
        .await
        .into_eyre()?;
    let button_event_message_pub = node
        .publisher::<ButtonEventMsg>("inputs/button_event_message")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    loop {
        tokio::select! {
            button_event = button_event_sub.recv_async() => {
                let button_event = button_event.into_eyre()?;

                let button_event = cdr::deserialize(&button_event.payload().to_bytes())
                    .wrap_err("deserialization failed")?;

                button_event_message_pub
                    .publish(&button_event)
                    .await
                    .into_eyre()?;


            }
        }
    }
}
