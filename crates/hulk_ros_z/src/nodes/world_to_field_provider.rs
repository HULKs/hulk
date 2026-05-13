use std::{future::pending, sync::Arc};

use color_eyre::Result;
use coordinate_systems::{Field, World};
use linear_algebra::Isometry2;
use ros_z::prelude::*;
use types::game_controller_state::GameControllerState;

use crate::IntoEyreResultExt;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("world_to_field_provider")
        .build()
        .await
        .into_eyre()?;
    let _game_controller_state_sub = node
        .subscriber::<GameControllerState>("game_controller_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _world_to_field_pub = node
        .publisher::<Isometry2<World, Field>>("world_to_field")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
