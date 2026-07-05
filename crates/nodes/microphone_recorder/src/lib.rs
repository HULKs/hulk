use std::future::pending;
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

    loop {
        let samples = block_in_place(|| microphones.retrying_read())?;
        microphones_samples_pub.publish(&samples).await?;
    }
}
