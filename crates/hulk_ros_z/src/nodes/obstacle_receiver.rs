use std::{future::pending, sync::Arc};

use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2};
use ros_z::prelude::*;
use types::{
    filtered_game_controller_state::FilteredGameControllerState, messages::IncomingMessage,
};

use crate::IntoEyreResultExt;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("obstacle_receiver")
        .build()
        .await
        .into_eyre()?;
    let _filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .build()
        .await
        .into_eyre()?;
    let _ground_to_field_sub = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")
        .build()
        .await
        .into_eyre()?;
    let _network_message_sub = node
        .subscriber::<IncomingMessage>("filtered_message")
        .build()
        .await
        .into_eyre()?;
    let _network_robot_obstacles_pub = node
        .publisher::<Vec<Point2<Ground>>>("network_robot_obstacles")
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
