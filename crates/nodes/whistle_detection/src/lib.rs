use std::{future::pending, sync::Arc};

use color_eyre::Result;

use ros_z::{IntoEyreResultExt, prelude::*};
use types::{
    parameters::WhistleDetectionParameters,
    samples::Samples,
    whistle::{DetectionInfo, Whistle},
};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("whistle_detection")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<WhistleDetectionParameters>("whistle_detection")
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
