use std::{boxed::Box, future::Future, pin::Pin};
use std::{sync::Arc, time::Duration};

use color_eyre::Result;

use ros_z::{prelude::*, qos::QosDurability};
use types::{
    behavior_tree::NodeTrace,
    field_dimensions::FieldDimensions,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    parameters::BehaviorParameters,
    path_obstacles::PathObstacle,
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("behavior_node").build().await?;

    let _parameters = node.bind_parameter_as::<BehaviorParameters>("behavior_node")?;
    let _field_dimensions_sub = node
        .subscriber::<FieldDimensions>("field_dimensions")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
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
    let primary_state_sub = node
        .subscriber::<PrimaryState>("primary_state")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let motion_command_pub = node
        .publisher::<MotionCommand>("motion_command")?
        .build()
        .await?;

    loop {
        let primary_state = primary_state_sub.recv().await?;
        let motion_command = motion_command_for_primary_state(primary_state);

        motion_command_pub.publish(&motion_command).await?;
    }
}

fn motion_command_for_primary_state(primary_state: PrimaryState) -> MotionCommand {
    match primary_state {
        PrimaryState::Safe
        | PrimaryState::Stop
        | PrimaryState::Penalized
        | PrimaryState::Finished => MotionCommand::Damping,
        PrimaryState::Initial | PrimaryState::Ready | PrimaryState::Set => MotionCommand::Prepare,
        PrimaryState::Playing => MotionCommand::Stand {
            head: HeadMotion::Center {
                image_region_target: ImageRegion::Center,
            },
        },
        PrimaryState::Custom => MotionCommand::Custom,
    }
}
