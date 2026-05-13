use std::{future::pending, sync::Arc};

use color_eyre::Result;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{
    parameters::WhistleDetectionParameters,
    samples::Samples,
    whistle::{DetectionInfo, Whistle},
};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub parameters: WhistleDetectionParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("whistle_detection")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("whistle_detection")
        .into_eyre()?;
    let _samples_sub = node
        .subscriber::<Samples>("samples")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    // TODO: restructure type layout here, do not use blank tuples
    // let _audio_spectrums_pub = node
    //     .publisher::<Vec<Vec<(f32, f32)>>>("audio_spectrums")
    //     .build()
    //     .await
    //     .into_eyre()?;
    let _detection_infos_pub = node
        .publisher::<Vec<DetectionInfo>>("detection_infos")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _detected_whistle_pub = node
        .publisher::<Whistle>("detected_whistle")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
