use std::sync::Arc;

use color_eyre::{Result, eyre::Context as _};

use booster::FallDownState;
use ros_z::{IntoEyreResultExt, prelude::*};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("fall_down_state_receiver")
        .build()
        .await
        .into_eyre()?;

    let zenoh_session = ctx.session();

    let fall_down_state_sub = zenoh_session
        .declare_subscriber("rt/fall_down")
        .await
        .into_eyre()?;
    let fall_down_state_pub = node
        .publisher::<FallDownState>("inputs/fall_down_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    loop {
        tokio::select! {
            fall_down_state = fall_down_state_sub.recv_async() => {
                let fall_down_state = fall_down_state.into_eyre()?;

                let deserialized_sample = cdr::deserialize(&fall_down_state.payload().to_bytes())
                    .wrap_err("deserialization failed")?;
                fall_down_state_pub
                    .publish(&deserialized_sample)
                    .await
                    .into_eyre()?;

             }
        }
    }
}
