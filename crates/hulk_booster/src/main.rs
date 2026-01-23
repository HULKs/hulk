use color_eyre::Result;
use tokio::task::JoinSet;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let mut set = JoinSet::new();

    set.spawn(control::sensor_data_receiver::run());
    // set.spawn(control::motion::booster_walking::run());

    let res = set.join_next().await.expect("at least one task");
    match res {
        Ok(Ok(())) => {
            tracing::warn!("Task finished unexpectedly");
        }
        Ok(Err(e)) => {
            tracing::error!("Task failed: {e:?}");
        }
        Err(e) => {
            tracing::error!("Task panicked: {e:?}");
        }
    }

    Ok(())
}
