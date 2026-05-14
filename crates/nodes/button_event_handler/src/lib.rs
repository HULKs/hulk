use std::{future::pending, sync::Arc};

use color_eyre::Result;

use booster::ButtonEventMsg;
use ros_z::{IntoEyreResultExt, prelude::*};
use types::buttons::{ButtonPressType, Buttons};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("button_event_handler")
        .build()
        .await
        .into_eyre()?;
    let _maybe_button_event_sub = node
        .subscriber::<ButtonEventMsg>("inputs/button_event_message")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _buttons_pub = node
        .publisher::<Buttons<Option<ButtonPressType>>>("buttons")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
