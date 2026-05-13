use std::{future::pending, sync::Arc};

use booster::FallDownState;
use color_eyre::Result;
use ros_z::prelude::*;

use crate::IntoEyreResultExt;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("fall_down_state_receiver")
        .build()
        .await
        .into_eyre()?;
    let _fall_down_state_pub = node
        .publisher::<FallDownState>("fall_down_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
