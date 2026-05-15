use std::sync::Arc;

use color_eyre::{Result, eyre::Context as _};

use booster::Odometer;
use ros_z::prelude::*;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("odometer_bridge").build().await?;

    let zenoh_session = ctx.session();

    let odometer_sub = zenoh_session
        .declare_subscriber("rt/odometer")
        .await
        .map_err(|error| color_eyre::eyre::eyre!("{error}"))?;
    let odometer_pub = node
        .publisher::<Odometer>("inputs/odometer")?
        .build()
        .await?;

    loop {
        tokio::select! {
            odometer = odometer_sub.recv_async() => {
                let odometer = odometer.map_err(|error| color_eyre::eyre::eyre!("{error}"))?;

                let odometer = cdr::deserialize(&odometer.payload().to_bytes())
                    .wrap_err("deserialization failed")?;

                odometer_pub
                    .publish(&odometer)
                    .await?;


            }
        }
    }
}
