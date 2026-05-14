use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use ros_z::prelude::*;
use types::{
    behavior_tree::NodeTrace, field_dimensions::FieldDimensions, motion_command::MotionCommand,
    parameters::BehaviorParameters, path_obstacles::PathObstacle, world_state::WorldState,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub field_dimensions: FieldDimensions,
    pub parameters: BehaviorParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("behavior_node").build().await?;

    let _parameters = node.bind_parameter_as::<Parameters>("behavior_node")?;
    let _world_state_sub = node
        .subscriber::<WorldState>("world_state")?
        .build()
        .await?;
    let _behavior_trace_pub = node
        .publisher::<NodeTrace>("behavior/trace")?
        .build()
        .await?;
    let _behavior_tree_layout_pub = node
        .publisher::<NodeTrace>("behavior/tree_layout")?
        .build()
        .await?;
    let _time_since_last_switch_pub = node
        .publisher::<Duration>("behavior/time_since_last_switch")?
        .build()
        .await?;
    let _path_obstacles_output_pub = node
        .publisher::<Vec<PathObstacle>>("path_obstacles")?
        .build()
        .await?;
    let _motion_command_pub = node
        .publisher::<MotionCommand>("motion_command")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
