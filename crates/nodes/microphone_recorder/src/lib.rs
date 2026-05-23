use std::sync::Arc;

use color_eyre::Result;

use microphones::{parameters::Parameters as MicrophonesParameters, reader::Microphones};
use ros_z::{context::Context, parameter::NodeParametersExt};
use types::samples::Samples;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("microphone_recorder").build().await?;

    let parameters = node.bind_parameter_as::<MicrophonesParameters>("microphone_recorder")?;

    let microphones_samples_pub = node
        .publisher::<Samples>("inputs/microphones_samples")?
        .build()
        .await?;

    let parameters_snapshot = parameters.snapshot();
    let parameters = parameters_snapshot.typed();
    let mut microphones = Microphones::new(parameters.clone())?;

    loop {
        let samples = microphones.retrying_read()?;
        microphones_samples_pub.publish(&samples).await?;
    }
}
