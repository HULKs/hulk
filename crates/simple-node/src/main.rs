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

    let namespace = "HULK10";
    let session = Session::create(namespace)
        .await
        .wrap_err("failed to create session")?;

    let node = session
        .create_node("low_state_fake")
        .build()
        .await
        .wrap_err("failed to create node")?;

    let publisher = node
        .create_publisher("booster/low_state")
        .build()
        .await
        .wrap_err("failed to create publisher")?;

    let (parameter, parameter_driver) = node
        .declare_parameter::<f32>("my_value")
        .build()
        .await
        .wrap_err("failed to declare parameter")?;
    tokio::spawn(parameter_driver);

    loop {
        let my_value = &*parameter.get().await;
        let value = LowState {
            imu_state: ImuState {
                roll_pitch_yaw: vector![*my_value, *my_value, *my_value],
                angular_velocity: vector![0.0, 0.0, 0.0],
                linear_acceleration: vector![0.0, 0.0, 0.0],
            },
            motor_state_parallel: vec![Default::default(); 22],
            motor_state_serial: vec![Default::default(); 22],
        };
        // info!("Publishing low_state: {value:?}");
        publisher
            .put(&value)
            .await
            .wrap_err("failed to put low_state to the publisher")?;
        sleep(Duration::from_secs(1)).await;
    }
}
