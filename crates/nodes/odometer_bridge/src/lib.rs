use std::sync::Arc;
use std::{boxed::Box, future::Future, pin::Pin};

use color_eyre::{
    Result,
    eyre::{Context as _, eyre},
};

use booster::Odometer;
use ros_z::prelude::*;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("odometer_bridge").build().await?;

    let zenoh_session = ctx.session();

    let odometer_sub = zenoh_session
        .declare_subscriber("rt/odometer_state")
        .await
        .map_err(|error| eyre!("{error}"))?;
    let odometer_pub = node
        .publisher::<Odometer>("inputs/odometer")?
        .build()
        .await?;

    loop {
        tokio::select! {
            odometer = odometer_sub.recv_async() => {
                let odometer = odometer.map_err(|error| eyre!("{error}"))?;

                let odometer = cdr::deserialize(&odometer.payload().to_bytes())
                    .wrap_err("deserialization failed")?;

                odometer_pub
                    .publish(&odometer)
                    .await?;


            }
        }
    }
}
