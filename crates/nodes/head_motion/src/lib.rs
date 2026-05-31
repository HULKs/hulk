use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster::MotorState;
use kinematics::joints::{Joints, head::HeadJoints};
use ros_z::prelude::*;
use types::{motion_command::MotionCommand, parameters::HeadMotionParameters};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub parameters: HeadMotionParameters,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("head_motion").build().await?;

    let _parameters = node.bind_parameter_as::<Parameters>("head_motion")?;
    let _look_around_target_joints_sub = node
        .subscriber::<HeadJoints<f32>>("look_around_target_joints")?
        .build()
        .await?;
    let _look_at_sub = node
        .subscriber::<HeadJoints<f32>>("look_at")?
        .build()
        .await?;
    let _motor_states_sub = node
        .subscriber::<Joints<MotorState>>("inputs/serial_motor_states")?
        .build()
        .await?;
    let _motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")?
        .build()
        .await?;
    let _head_joints_command_pub = node
        .publisher::<HeadJoints<f32>>("head_joints_command")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
