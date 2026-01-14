use booster::LowState;
use color_eyre::Result;
use hulkz::Session;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let rec = rerun::RecordingStreamBuilder::new("simple-viewer").spawn()?;

    let session = Session::new().await?;

    let mut low_state = session
        .stream::<LowState>("HULK10/booster/low_state")
        .await?;

    while let Ok(low_state) = low_state.recv_async().await {
        let accelerometer = low_state.imu_state.linear_acceleration;
        rec.log(
            "booster/imu/accelerometer",
            &rerun::archetypes::Scalars::new([
                accelerometer.x(),
                accelerometer.y(),
                accelerometer.z(),
            ]),
        )?;
        let rpy = low_state.imu_state.roll_pitch_yaw;
        rec.log(
            "booster/imu/roll_pitch_yaw",
            &rerun::archetypes::Scalars::new([rpy.x(), rpy.y(), rpy.z()]),
        )?;
    }

    Ok(())
}
