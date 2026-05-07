use std::{future::pending, sync::Arc};

use booster::MotorState;
use color_eyre::Result;
use kinematics::{joints::Joints, robot_kinematics::RobotKinematics};
use ros_z::prelude::*;

use crate::IntoEyreResultExt;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("kinematics_provider")
        .build()
        .await
        .into_eyre()?;
    let _serial_motor_states_sub = node
        .subscriber::<Joints<MotorState>>("serial_motor_states")
        .build()
        .await
        .into_eyre()?;
    let _robot_kinematics_pub = node
        .publisher::<RobotKinematics>("robot_kinematics")
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
