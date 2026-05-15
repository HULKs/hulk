use std::{future::pending, sync::Arc};

use color_eyre::Result;

use booster::ButtonEventMsg;
use ros_z::prelude::*;
use types::buttons::{ButtonPressType, Buttons};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("button_event_handler").build().await?;
    let _maybe_button_event_sub = node
        .subscriber::<ButtonEventMsg>("inputs/button_event_message")?
        .build()
        .await?;
    let _buttons_pub = node
        .publisher::<Buttons<Option<ButtonPressType>>>("buttons")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
