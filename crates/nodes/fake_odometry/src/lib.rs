use std::{future::pending, sync::Arc};

use color_eyre::Result;
use nalgebra as na;

use ros_z::{IntoEyreResultExt, prelude::*};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("fake_odometry").build().await.into_eyre()?;
    let _current_odometry_to_last_odometry_pub = node
        .publisher::<na::Isometry2<f32>>("current_odometry_to_last_odometry")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
