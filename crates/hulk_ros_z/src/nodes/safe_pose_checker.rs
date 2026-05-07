use std::{future::pending, sync::Arc};

use booster::{ImuState, MotorState};
use color_eyre::Result;
use coordinate_systems::Robot;
use kinematics::joints::Joints;
use linear_algebra::Vector3;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub prep_mode_serial_motor_states: Joints<MotorState>,
    pub prep_mode_imu_state: ImuState,
    pub joint_position_threshold: f32,
    pub joint_velocity_threshold: f32,
    pub angular_velocity_threshold: f32,
    pub linear_acceleration_threshold: f32,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("safe_pose_checker")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("safe_pose_checker")
        .into_eyre()?;
    let _imu_state_sub = node
        .subscriber::<ImuState>("imu_state")
        .build()
        .await
        .into_eyre()?;
    let _serial_motor_states_sub = node
        .subscriber::<Joints<MotorState>>("serial_motor_states")
        .build()
        .await
        .into_eyre()?;
    let _joint_position_difference_to_safe_pub = node
        .publisher::<Joints>("joint_position_difference_to_safe")
        .build()
        .await
        .into_eyre()?;
    let _joint_velocities_difference_to_safe_pub = node
        .publisher::<Joints>("joint_velocities_difference_to_safe")
        .build()
        .await
        .into_eyre()?;
    let _angular_velocities_difference_to_safe_pub = node
        .publisher::<Vector3<Robot>>("angular_velocities_difference_to_safe")
        .build()
        .await
        .into_eyre()?;
    let _linear_accelerations_difference_to_safe_pub = node
        .publisher::<Vector3<Robot>>("linear_accelerations_difference_to_safe")
        .build()
        .await
        .into_eyre()?;
    let _is_safe_pose_pub = node
        .publisher::<bool>("is_safe_pose")
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
