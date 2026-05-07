use std::{future::pending, sync::Arc};

use booster::{ImuState, MotorState};
use color_eyre::Result;
use kinematics::joints::Joints;
use ros_z::prelude::*;

use crate::IntoEyreResultExt;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("sensor_data_receiver")
        .build()
        .await
        .into_eyre()?;
    let _imu_state_pub = node
        .publisher::<ImuState>("imu_state")
        .build()
        .await
        .into_eyre()?;
    let _serial_motor_states_pub = node
        .publisher::<Joints<MotorState>>("serial_motor_states")
        .build()
        .await
        .into_eyre()?;
    let _parallel_motor_states_pub = node
        .publisher::<Joints<MotorState>>("parallel_motor_states")
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
