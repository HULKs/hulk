use std::{collections::HashSet, future::pending, sync::Arc};

use color_eyre::Result;
use hsl_network_messages::PlayerNumber;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{
    buttons::{ButtonPressType, Buttons},
    filtered_game_controller_state::FilteredGameControllerState,
    primary_state::PrimaryState,
};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub injected_primary_state: Option<PrimaryState>,
    pub player_number: PlayerNumber,
    pub recorded_primary_states: HashSet<PrimaryState>,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("primary_state_filter")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("primary_state_filter")
        .into_eyre()?;
    let _buttons_sub = node
        .subscriber::<Buttons<Option<ButtonPressType>>>("buttons")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _is_safe_pose_sub = node
        .subscriber::<bool>("is_safe_pose")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _primary_state_pub = node
        .publisher::<PrimaryState>("primary_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
