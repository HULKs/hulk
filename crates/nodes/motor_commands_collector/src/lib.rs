use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc};

use color_eyre::Result;

use kinematics::joints::Joints;
use ros_z::prelude::*;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("motor_commands_collector").build().await?;
    let _collected_target_joint_positions_pub = node
        .publisher::<Joints<f32>>("collected_target_joint_positions")
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
