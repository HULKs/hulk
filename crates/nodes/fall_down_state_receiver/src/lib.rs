use std::sync::Arc;
use std::{boxed::Box, future::Future, pin::Pin};

use color_eyre::{
    Result,
    eyre::{Context as _, eyre},
};

use booster::FallDownState;
use ros_z::prelude::*;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("fall_down_state_receiver").build().await?;

    let zenoh_session = ctx.session();

    let fall_down_state_sub = zenoh_session
        .declare_subscriber("rt/fall_down")
        .await
        .map_err(|error| eyre!("{error}"))?;
    let fall_down_state_pub = node
        .publisher::<FallDownState>("inputs/fall_down_state")
        .build()
        .await?;

    loop {
        tokio::select! {
            fall_down_state = fall_down_state_sub.recv_async() => {
                let fall_down_state = fall_down_state.map_err(|error| eyre!("{error}"))?;

                let deserialized_sample = cdr::deserialize(&fall_down_state.payload().to_bytes())
                    .wrap_err("deserialization failed")?;
                fall_down_state_pub
                    .publish(&deserialized_sample)
                    .await?;

             }
        }
    }
}
