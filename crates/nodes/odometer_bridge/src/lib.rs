use std::sync::Arc;

use color_eyre::{Result, eyre::Context as _};

use booster::Odometer;
use ros_z::{IntoEyreResultExt, prelude::*};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("odometer_bridge")
        .build()
        .await
        .into_eyre()?;

    let zenoh_session = ctx.session();

    let odometer_sub = zenoh_session
        .declare_subscriber("rt/odometer")
        .await
        .into_eyre()?;
    let odometer_pub = node
        .publisher::<Odometer>("inputs/odometer")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    loop {
        tokio::select! {
            odometer = odometer_sub.recv_async() => {
                let odometer = odometer.into_eyre()?;

                let odometer = cdr::deserialize(&odometer.payload().to_bytes())
                    .wrap_err("deserialization failed")?;

                odometer_pub
                    .publish(&odometer)
                    .await
                    .into_eyre()?;


            }
        }
    }
}
