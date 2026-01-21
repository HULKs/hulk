use std::time::Duration;

use booster::{ImuState, LowState};
use color_eyre::{eyre::Context as _, Result};
use hulkz::Session;
use linear_algebra::vector;
use tokio::time::sleep;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    info!("Starting Node...");

    let session = Session::new().await?;

    let publisher = session
        .publish("booster/low_state")
        .await
        .wrap_err("failed to create publisher")?;

    loop {
        let value = LowState {
            imu_state: ImuState {
                roll_pitch_yaw: vector![0.0, 0.0, 0.0],
                angular_velocity: vector![0.0, 0.0, 0.0],
                linear_acceleration: vector![0.0, 0.0, 0.0],
            },
            motor_state_parallel: Vec::new(),
            motor_state_serial: Vec::new(),
        };
        info!("Publishing low_state: {value:?}");
        publisher
            .put(&value)
            .await
            .wrap_err("failed to put low_state to the publisher")?;
        sleep(Duration::from_secs(1)).await;
    }
}
