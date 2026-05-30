use std::sync::Arc;
use std::{boxed::Box, future::Future, pin::Pin};

use color_eyre::{
    Result,
    eyre::{Context as _, eyre},
};

use booster::Odometer;
use ros_z::prelude::*;
use ros_z_streams::CreateAnnouncingPublisher;

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
        .announcing_publisher::<Odometer>("inputs/odometer")
        .await?;

    loop {
        tokio::select! {
            odometer = odometer_sub.recv_async() => {
                let odometer = odometer.map_err(|error| eyre!("{error}"))?;

                let odometer = cdr::deserialize(&odometer.payload().to_bytes())
                    .wrap_err("deserialization failed")?;

                odometer_pub
                    .announce(node.clock().now())
                    .await?
                    .publish(&odometer)
                    .await?;
            }
        }
    }
}
