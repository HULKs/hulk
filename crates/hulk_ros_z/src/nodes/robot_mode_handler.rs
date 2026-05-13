use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{
    buttons::{ButtonPressType, Buttons},
    primary_state::PrimaryState,
};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub wait_before_prepare: Duration,
    pub remote_stop_toggle: bool,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("robot_mode_handler")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("robot_mode_handler")
        .into_eyre()?;
    let _primary_state_sub = node
        .subscriber::<PrimaryState>("primary_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _buttons_sub = node
        .subscriber::<Buttons<Option<ButtonPressType>>>("buttons")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    // TODO: booster_sdk is not owned by HULKs, we cannot directly implement Message for that...
    // let _robot_mode_pub = node
    //     .publisher::<RobotMode>("robot_mode")
    //     .build()
    //     .await
    //     .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
