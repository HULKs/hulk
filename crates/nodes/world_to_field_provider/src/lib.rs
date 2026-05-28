use std::{f32::consts::PI, sync::Arc};

use color_eyre::Result;

use coordinate_systems::{Field, World};
use linear_algebra::Isometry2;
use ros_z::prelude::*;
use types::{
    field_dimensions::GlobalFieldSide, game_controller_state::GameControllerState,
    time_wrapper::TimeWrapper,
};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("world_to_field_provider").build().await?;
    let game_controller_state_sub = node
        .subscriber::<TimeWrapper<Option<GameControllerState>>>("game_controller_state")?
        .build()
        .await?;

    let world_to_field_pub = node
        .publisher::<TimeWrapper<Isometry2<World, Field>>>("world_to_field")?
        .build()
        .await?;

    loop {
        let game_controller_state_wrapper = game_controller_state_sub.recv().await?;

        let TimeWrapper {
            time,
            inner: Some(game_controller_state),
        } = game_controller_state_wrapper
        else {
            continue;
        };

        let world_to_field = TimeWrapper {
            time,
            inner: match game_controller_state.global_field_side {
                GlobalFieldSide::Home => Isometry2::identity(),
                GlobalFieldSide::Away => Isometry2::rotation(PI),
            },
        };

        world_to_field_pub.publish(&world_to_field).await?;
    }
}
