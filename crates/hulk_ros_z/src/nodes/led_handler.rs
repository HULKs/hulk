use std::{future::pending, sync::Arc};

use color_eyre::Result;
use ros_z::prelude::*;
use types::primary_state::PrimaryState;

use crate::IntoEyreResultExt;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("led_handler").build().await.into_eyre()?;
    let _primary_state_sub = node
        .subscriber::<PrimaryState>("primary_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
