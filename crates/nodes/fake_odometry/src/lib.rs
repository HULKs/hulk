use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc};

use color_eyre::Result;
use nalgebra as na;

use ros_z::prelude::*;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("fake_odometry").build().await?;
    let _current_odometry_to_last_odometry_pub = node
        .publisher::<na::Isometry2<f32>>("current_odometry_to_last_odometry")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
