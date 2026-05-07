use std::{future::pending, sync::Arc};

use color_eyre::Result;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub cycler_frequency: f32,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("trigger").build().await.into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("trigger")
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
