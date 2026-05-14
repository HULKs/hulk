use std::{future::pending, sync::Arc};

use color_eyre::Result;

use booster::MotorState;
use kinematics::{joints::Joints, robot_kinematics::RobotKinematics};
use ros_z::{IntoEyreResultExt, prelude::*};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("kinematics_provider")
        .build()
        .await
        .into_eyre()?;
    let _serial_motor_states_sub = node
        .subscriber::<Joints<MotorState>>("inputs/serial_motor_states")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _robot_kinematics_pub = node
        .publisher::<RobotKinematics>("robot_kinematics")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
