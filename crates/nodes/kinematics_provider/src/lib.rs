use std::{future::pending, sync::Arc};

use color_eyre::Result;

use booster::MotorState;
use kinematics::{joints::Joints, robot_kinematics::RobotKinematics};
use ros_z::prelude::*;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("kinematics_provider").build().await?;
    let _serial_motor_states_sub = node
        .subscriber::<Joints<MotorState>>("inputs/serial_motor_states")?
        .build()
        .await?;
    let _robot_kinematics_pub = node
        .publisher::<RobotKinematics>("robot_kinematics")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
