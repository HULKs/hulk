use std::{future::pending, sync::Arc};

use color_eyre::Result;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{filtered_whistle::FilteredWhistle, whistle::Whistle};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub buffer_length: usize,
    pub minimum_detections: usize,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("whistle_filter")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("whistle_filter")
        .into_eyre()?;
    let _detected_whistle_sub = node
        .subscriber::<Whistle>("detected_whistle")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _filtered_whistle_pub = node
        .publisher::<FilteredWhistle>("filtered_whistle")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
