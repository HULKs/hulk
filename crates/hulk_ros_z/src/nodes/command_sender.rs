use std::{future::pending, sync::Arc};

use booster::{LowCommand, MotorCommandParameters};
use color_eyre::Result;
use kinematics::joints::Joints;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub prepare_motor_command_parameters: MotorCommandParameters,
    pub walk_motor_command_parameters: MotorCommandParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("command_sender")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("command_sender")
        .into_eyre()?;
    let _collected_target_joint_positions_sub = node
        .subscriber::<Joints<f32>>("collected_target_joint_positions")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _low_command_pub = node
        .publisher::<LowCommand>("low_command")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
