use std::{future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster::{ImuState, MotorState};
use coordinate_systems::Robot;
use kinematics::joints::Joints;
use linear_algebra::Vector3;
use ros_z::prelude::*;

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
    let node = ctx.create_node("safe_pose_checker").build().await?;

    let _parameters = node.bind_parameter_as::<Parameters>("safe_pose_checker")?;
    let _imu_state_sub = node
        .subscriber::<ImuState>("inputs/imu_state")?
        .build()
        .await?;
    let _serial_motor_states_sub = node
        .subscriber::<Joints<MotorState>>("inputs/serial_motor_states")?
        .build()
        .await?;
    let _joint_position_difference_to_safe_pub = node
        .publisher::<Joints>("joint_position_difference_to_safe")?
        .build()
        .await?;
    let _joint_velocities_difference_to_safe_pub = node
        .publisher::<Joints>("joint_velocities_difference_to_safe")?
        .build()
        .await?;
    let _angular_velocities_difference_to_safe_pub = node
        .publisher::<Vector3<Robot>>("angular_velocities_difference_to_safe")?
        .build()
        .await?;
    let _linear_accelerations_difference_to_safe_pub = node
        .publisher::<Vector3<Robot>>("linear_accelerations_difference_to_safe")?
        .build()
        .await?;
    let _is_safe_pose_pub = node.publisher::<bool>("is_safe_pose")?.build().await?;

    pending::<()>().await;

    Ok(())
}
