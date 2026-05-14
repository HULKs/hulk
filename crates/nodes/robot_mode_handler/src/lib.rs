use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster_sdk_interface::GetRobotMode;
use ros_z::{IntoEyreResultExt, prelude::*};
use types::{
    buttons::{ButtonPressType, Buttons},
    primary_state::PrimaryState,
};

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
    let _get_robot_mode_client = node
        .create_service_client::<GetRobotMode>("services/get_robot_mode")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
