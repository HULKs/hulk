use std::{future::pending, sync::Arc};

use booster::{ButtonEventMsg};
use color_eyre::Result;
use ros_z::prelude::*;

use crate::IntoEyreResultExt;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("button_event_receiver").build().await.into_eyre()?;
    let _button_event_pub = node.publisher::<ButtonEventMsg>("button_event").build().await.into_eyre()?;

    pending::<()>().await;

    Ok(())
}
