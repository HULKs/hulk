use std::sync::Arc;
use std::{boxed::Box, future::Future, pin::Pin};

use color_eyre::Result;
use log::warn;

use microphones::{parameters::Parameters as MicrophonesParameters, reader::Microphones};
use ros_z::{context::Context, parameter::NodeParametersExt};
use tokio::task::block_in_place;
use types::samples::Samples;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("microphone_recorder").build().await?;

    let parameters = node.bind_parameter_as::<MicrophonesParameters>("microphone_recorder")?;

    let microphones_samples_pub = node
        .publisher::<Samples>("inputs/microphones_samples")
        .build()
        .await?;

    let parameters_snapshot = parameters.snapshot();
    let parameters = parameters_snapshot.typed();
    let mut microphones = match block_in_place(|| Microphones::new(parameters.clone())) {
        Ok(microphones) => microphones,
        Err(error) => {
            warn!("failed to create microphones: {error:#?}");
            return Ok(());
        }
    };

    let mut failed_attempts = 0u32;
    const ATTEMPTS_BEFORE_DEVICE_RESET: u32 = 3;

    loop {
        match block_in_place(|| microphones.read()) {
            Ok(samples) => {
                failed_attempts = 0;
                microphones_samples_pub.publish(&samples).await?;
            }
            Err(error) => {
                failed_attempts += 1;

                warn!("failed to read from microphones, attempt={failed_attempts}: {error:#?}");
                tokio::time::sleep(parameters.retry_sleep_duration).await;

                if failed_attempts >= ATTEMPTS_BEFORE_DEVICE_RESET {
                    failed_attempts = 0;

                    warn!(
                        "failed {ATTEMPTS_BEFORE_DEVICE_RESET} times consecutively, resetting the microphone device..."
                    );

                    if let Err(error) = microphones.recreate_device() {
                        warn!("failed to recreate microphone device: {error:#}");
                    }
                }
            }
        }
    }
}
