use std::{future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster::MotorState;
use kinematics::joints::{Joints, head::HeadJoints};
use ros_z::{IntoEyreResultExt, prelude::*};
use types::{motion_command::MotionCommand, parameters::HeadMotionParameters};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub parameters: HeadMotionParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("head_motion").build().await.into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("head_motion")
        .into_eyre()?;
    let _look_around_target_joints_sub = node
        .subscriber::<HeadJoints<f32>>("look_around_target_joints")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _look_at_sub = node
        .subscriber::<HeadJoints<f32>>("look_at")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _motor_states_sub = node
        .subscriber::<Joints<MotorState>>("inputs/serial_motor_states")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _head_joints_command_pub = node
        .publisher::<HeadJoints<f32>>("head_joints_command")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
