use std::{future::pending, sync::Arc};

use booster::Odometer;
use color_eyre::Result;
use ros_z::prelude::*;

use crate::IntoEyreResultExt;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("odometer_receiver")
        .build()
        .await
        .into_eyre()?;
    let _odometer_pub = node
        .publisher::<Odometer>("odometer")
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
