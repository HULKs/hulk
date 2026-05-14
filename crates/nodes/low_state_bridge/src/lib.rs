use std::sync::Arc;

use color_eyre::{Result, eyre::Context as _};

use booster::{ImuState, LowState, MotorState};
use kinematics::joints::Joints;
use ros_z::{IntoEyreResultExt, prelude::*};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("low_state_bridge")
        .build()
        .await
        .into_eyre()?;

    let zenoh_session = ctx.session();

    let low_state_sub = zenoh_session
        .declare_subscriber("rt/low_state")
        .await
        .into_eyre()?;

    let low_state_pub = node
        .publisher::<LowState>("inputs/low_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let imu_state_pub = node
        .publisher::<ImuState>("inputs/imu_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let serial_motor_states_pub = node
        .publisher::<Joints<MotorState>>("inputs/serial_motor_states")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let parallel_motor_states_pub = node
        .publisher::<Option<Joints<MotorState>>>("inputs/parallel_motor_states")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    loop {
        tokio::select! {
            low_state = low_state_sub.recv_async() => {
                let low_state = low_state.into_eyre()?;

                let low_state: LowState = cdr::deserialize(&low_state.payload().to_bytes())
                    .wrap_err("deserialization failed")?;

                let imu_state = low_state.imu_state;
                let serial_motor_states = low_state.serial_motor_states()?;
                let parallel_motor_states = low_state.parallel_motor_states().ok();

                low_state_pub
                    .publish(&low_state)
                    .await
                    .into_eyre()?;
                imu_state_pub
                    .publish(&imu_state)
                    .await
                    .into_eyre()?;
                serial_motor_states_pub
                    .publish(&serial_motor_states)
                    .await
                    .into_eyre()?;
                parallel_motor_states_pub
                    .publish(&parallel_motor_states)
                    .await
                    .into_eyre()?;

            }
        }
    }
}
