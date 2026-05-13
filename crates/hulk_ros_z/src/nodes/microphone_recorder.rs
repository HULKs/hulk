use std::{future::pending, sync::Arc};

use color_eyre::Result;
use ros_z::prelude::*;
use types::samples::Samples;

use crate::IntoEyreResultExt;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("microphone_recorder")
        .build()
        .await
        .into_eyre()?;
    let _samples_pub = node
        .publisher::<Samples>("samples")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
