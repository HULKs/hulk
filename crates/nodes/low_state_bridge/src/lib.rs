use std::sync::Arc;
use std::{boxed::Box, future::Future, pin::Pin};

use color_eyre::{Result, eyre::Context as _};

use booster::{ImuState, LowState, MotorState};
use kinematics::joints::Joints;
use ros_z::prelude::*;
use ros_z_streams::CreateAnnouncingPublisher;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("low_state_bridge").build().await?;

    let zenoh_session = ctx.session();

    let low_state_sub = zenoh_session
        .declare_subscriber("rt/low_state")
        .await
        .map_err(|error| color_eyre::eyre::eyre!("{error}"))?;

    let low_state_pub = node
        .publisher::<LowState>("inputs/low_state")
        .build()
        .await?;
    let imu_state_pub = node
        .announcing_publisher::<ImuState>("inputs/imu_state")
        .await?;
    let serial_motor_states_pub = node
        .publisher::<Joints<MotorState>>("inputs/serial_motor_states")
        .build()
        .await?;
    let parallel_motor_states_pub = node
        .publisher::<Option<Joints<MotorState>>>("inputs/parallel_motor_states")
        .build()
        .await?;

    loop {
        tokio::select! {
            low_state = low_state_sub.recv_async() => {
                let low_state = low_state.map_err(|error| color_eyre::eyre::eyre!("{error}"))?;

                let low_state: LowState = cdr::deserialize(&low_state.payload().to_bytes())
                    .wrap_err("deserialization failed")?;

                let imu_state = low_state.imu_state;
                let serial_motor_states = low_state.serial_motor_states()?;
                let parallel_motor_states = low_state.parallel_motor_states().ok();
                let source_time = node.clock().now();
                let pending_imu_state = imu_state_pub.announce(source_time).await?;

                low_state_pub
                    .publish(&low_state)
                    .await?;
                pending_imu_state.publish(&imu_state).await?;
                serial_motor_states_pub
                    .publish(&serial_motor_states)
                    .await?;
                parallel_motor_states_pub
                    .publish(&parallel_motor_states)
                    .await?;

            }
        }
    }
}
