use std::{
    future::pending,
    sync::Arc,
    time::{Duration, SystemTime},
};

use color_eyre::Result;

use coordinate_systems::{Field, Ground};
use linear_algebra::{Point2, Pose2, Vector2};
use ros_z::{prelude::*, qos::QosDurability};
use serde::{Deserialize, Serialize};
use types::{
    behavior_tree::NodeTrace,
    field_dimensions::{FieldDimensions, Side},
    motion_command::{BodyMotion, HeadMotion, MotionCommand},
    motion_type::MotionType,
    parameters::BehaviorParameters,
    path_obstacles::PathObstacle,
    world_state::WorldState,
};
use voronoi::VoronoiGrid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastBall {
    pub position: Point2<Field>,
    pub velocity: Vector2<Ground>,
    pub age: SystemTime,
    pub field_side: Side,
}

#[derive(Debug, Clone, Serialize)]
pub struct Blackboard {
    pub field_dimensions: FieldDimensions,
    pub free_kick_obstacle_radius: f32,
    pub parameters: BehaviorParameters,
    pub world_state: WorldState,

    pub path_obstacles_output: Vec<PathObstacle>,
    pub time_since_last_switch: Duration,
    pub direction_difference: f32,
    pub voronoi_inputs: Vec<Pose2<Field>>,

    pub ball: Option<LastBall>,
    pub last_ball: Option<LastBall>,
    pub last_close_enough_to_kick: bool,
    pub last_motion_command: MotionCommand,
    pub last_motion_switch_time: SystemTime,
    pub last_motion_type: Option<MotionType>,

    pub is_injected_motion_command: bool,
    pub walk_position: Option<Point2<Ground>>,
    pub body_motion: Option<BodyMotion>,
    pub head_motion: Option<HeadMotion>,
    pub voronoi_map: Option<VoronoiGrid>,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
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
    let _motion_command_pub = node
        .publisher::<MotionCommand>("motion_command")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
