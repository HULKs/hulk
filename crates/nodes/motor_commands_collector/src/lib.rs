use std::{future::pending, sync::Arc};

use color_eyre::Result;

use kinematics::joints::{Joints, head::HeadJoints};
use ros_z::prelude::*;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("motor_commands_collector").build().await?;
    let _head_target_joints_positions_sub = node
        .subscriber::<HeadJoints<f32>>("head_joints_command")?
        .build()
        .await?;
    let _collected_target_joint_positions_pub = node
        .publisher::<Joints<f32>>("collected_target_joint_positions")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
