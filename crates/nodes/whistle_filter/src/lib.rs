use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use ros_z::prelude::*;
use types::{filtered_whistle::FilteredWhistle, whistle::Whistle};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub buffer_length: usize,
    pub minimum_detections: usize,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("whistle_filter").build().await?;

    let _parameters = node.bind_parameter_as::<Parameters>("whistle_filter")?;
    let _detected_whistle_sub = node
        .subscriber::<Whistle>("detected_whistle")?
        .build()
        .await?;
    let _filtered_whistle_pub = node
        .publisher::<FilteredWhistle>("filtered_whistle")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
