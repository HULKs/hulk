use std::{collections::HashSet, future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use hsl_network_messages::PlayerNumber;
use ros_z::{prelude::*, qos::QosDurability};
use types::{
    buttons::{ButtonPressType, Buttons},
    filtered_game_controller_state::FilteredGameControllerState,
    primary_state::PrimaryState,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub injected_primary_state: Option<PrimaryState>,
    pub recorded_primary_states: HashSet<PrimaryState>,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("primary_state_filter").build().await?;

    let _parameters = node.bind_parameter_as::<Parameters>("primary_state_filter")?;
    let _player_number_sub = node
        .subscriber::<PlayerNumber>("player_number")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let _buttons_sub = node
        .subscriber::<Buttons<Option<ButtonPressType>>>("buttons")?
        .build()
        .await?;
    let _filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")?
        .build()
        .await?;
    let _is_safe_pose_sub = node.subscriber::<bool>("is_safe_pose")?.build().await?;
    let _primary_state_pub = node
        .publisher::<PrimaryState>("primary_state")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
