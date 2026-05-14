use std::{future::pending, sync::Arc};

use color_eyre::Result;

use booster::ImuState;
use coordinate_systems::{Ground, Robot};
use kinematics::robot_kinematics::RobotKinematics;
use linear_algebra::Isometry3;
use ros_z::{IntoEyreResultExt, prelude::*};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("ground_provider")
        .build()
        .await
        .into_eyre()?;
    let _robot_kinematics_sub = node
        .subscriber::<RobotKinematics>("robot_kinematics")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _imu_state_sub = node
        .subscriber::<ImuState>("inputs/imu_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _robot_to_ground_pub = node
        .publisher::<Isometry3<Robot, Ground>>("robot_to_ground")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _ground_to_robot_pub = node
        .publisher::<Isometry3<Ground, Robot>>("ground_to_robot")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
