use std::sync::Arc;

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use microphones::{parameters::Parameters as MicrophonesParameters, reader::Microphones};
use ros_z::{IntoEyreResultExt, Message, context::Context, parameter::NodeParametersExt};
use types::samples::Samples;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
struct Parameters {
    microphones: MicrophonesParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("microphone_recorder")
        .build()
        .await
        .into_eyre()?;

    let parameters = node
        .bind_parameter_as::<Parameters>("microphone_recorder")
        .into_eyre()?;

    let microphones_samples_pub = node
        .publisher::<Samples>("inputs/microphones_samples")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    let parameters_snapshot = parameters.snapshot();
    let parameters = parameters_snapshot.typed();
    let mut microphones = Microphones::new(parameters.microphones.clone())?;

    loop {
        let samples = microphones.retrying_read().into_eyre()?;
        microphones_samples_pub
            .publish(&samples)
            .await
            .into_eyre()?;
    }
}
