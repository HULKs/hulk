use std::{future::pending, sync::Arc};

use color_eyre::Result;
use ros_z::prelude::*;
use types::messages::IncomingMessage;

use crate::IntoEyreResultExt;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("message_receiver")
        .build()
        .await
        .into_eyre()?;
    let _message_pub = node
        .publisher::<IncomingMessage>("message")
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
