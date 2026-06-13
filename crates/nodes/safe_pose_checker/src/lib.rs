use std::sync::Arc;
use std::{boxed::Box, future::Future, pin::Pin};

use approx::AbsDiffEq;
use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster::{ImuState, JointsMotorState, MotorState};
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

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("safe_pose_checker").build().await?;

    let parameters = node
        .bind_parameter_as::<Parameters>("safe_pose_checker")
        .await?;
    let imu_state_sub = node
        .subscriber::<ImuState>("inputs/imu_state")?
        .build()
        .await?;
    let serial_motor_states_cache = node
        .create_cache::<Joints<MotorState>>("inputs/serial_motor_states", 10)?
        .build()
        .await?;
    let joint_position_difference_to_safe_pub = node
        .publisher::<Joints>("joint_position_difference_to_safe")?
        .build()
        .await?;
    let joint_velocities_difference_to_safe_pub = node
        .publisher::<Joints>("joint_velocities_difference_to_safe")?
        .build()
        .await?;
    let angular_velocities_difference_to_safe_pub = node
        .publisher::<Vector3<Robot>>("angular_velocities_difference_to_safe")?
        .build()
        .await?;
    let linear_accelerations_difference_to_safe_pub = node
        .publisher::<Vector3<Robot>>("linear_accelerations_difference_to_safe")?
        .build()
        .await?;
    let is_safe_pose_pub = node.publisher::<bool>("is_safe_pose")?.build().await?;
    loop {
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        let received_imu_state = imu_state_sub.recv_with_metadata().await?;

        let Some(serial_motor_states) =
            serial_motor_states_cache.get_nearest(received_imu_state.source_time)
        else {
            continue;
        };
        let imu_state = received_imu_state.into_message();

        let joint_position_difference_to_safe =
            serial_motor_states.positions() - parameters.prep_mode_serial_motor_states.positions();
        joint_position_difference_to_safe_pub
            .publish(&joint_position_difference_to_safe)
            .await?;

        let joint_velocities_difference_to_safe = serial_motor_states.velocities()
            - parameters.prep_mode_serial_motor_states.velocities();
        joint_velocities_difference_to_safe_pub
            .publish(&joint_velocities_difference_to_safe)
            .await?;

        let linear_accelerations_difference_to_safe =
            imu_state.linear_acceleration - parameters.prep_mode_imu_state.linear_acceleration;
        linear_accelerations_difference_to_safe_pub
            .publish(&linear_accelerations_difference_to_safe)
            .await?;

        let angular_velocities_difference_to_safe =
            imu_state.angular_velocity - parameters.prep_mode_imu_state.angular_velocity;
        angular_velocities_difference_to_safe_pub
            .publish(&angular_velocities_difference_to_safe)
            .await?;

        let motor_states_are_safe = motor_states_are_safe(
            &serial_motor_states,
            &parameters.prep_mode_serial_motor_states,
            parameters.joint_position_threshold,
            parameters.joint_velocity_threshold,
        );

        let imu_state_is_safe = imu_state_is_safe(
            &imu_state,
            &parameters.prep_mode_imu_state,
            parameters.angular_velocity_threshold,
            parameters.linear_acceleration_threshold,
        );

        let is_safe_pose = motor_states_are_safe && imu_state_is_safe;

        is_safe_pose_pub.publish(&is_safe_pose).await?;
    }
}

fn motor_states_are_safe(
    serial_motor_states: &Joints<MotorState>,
    prep_mode_serial_motor_states: &Joints<MotorState>,
    joint_position_threshold: f32,
    joint_velocity_threshold: f32,
) -> bool {
    serial_motor_states
        .into_iter()
        .zip(*prep_mode_serial_motor_states)
        .all(|(current_motor_state, safe_motor_state)| {
            current_motor_state
                .position
                .abs_diff_eq(&safe_motor_state.position, joint_position_threshold)
                && current_motor_state
                    .velocity
                    .abs_diff_eq(&safe_motor_state.velocity, joint_velocity_threshold)
        })
}

fn imu_state_is_safe(
    imu_state: &ImuState,
    prep_mode_imu_state: &ImuState,
    angular_velocity_threshold: f32,
    linear_acceleration_threshold: f32,
) -> bool {
    imu_state.angular_velocity.abs_diff_eq(
        &prep_mode_imu_state.angular_velocity,
        angular_velocity_threshold,
    ) && imu_state.linear_acceleration.abs_diff_eq(
        &prep_mode_imu_state.linear_acceleration,
        linear_acceleration_threshold,
    )
}
