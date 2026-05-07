use std::{
    collections::HashMap,
    future::pending,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use color_eyre::Result;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{game_controller_state::GameControllerState, messages::IncomingMessage};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub time_since_last_message_to_consider_ip_active: Duration,
    pub collision_alert_cooldown: Duration,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("game_controller_filter")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("game_controller_filter")
        .into_eyre()?;
    let _network_message_sub = node
        .subscriber::<IncomingMessage>("filtered_message")
        .build()
        .await
        .into_eyre()?;
    let _last_contact_pub = node
        .publisher::<HashMap<SocketAddr, SystemTime>>("game_controller_address_contacts_times")
        .build()
        .await
        .into_eyre()?;
    let _game_controller_state_pub = node
        .publisher::<GameControllerState>("game_controller_state")
        .build()
        .await
        .into_eyre()?;
    let _game_controller_address_pub = node
        .publisher::<SocketAddr>("game_controller_address")
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
