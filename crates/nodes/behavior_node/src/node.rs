use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use booster::FallDownState;
use color_eyre::Result;

use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Point2, Pose2, Vector2};
use ros_z::{prelude::*, qos::QosDurability};
use serde::{Deserialize, Serialize};
use types::{
    ball_position::HypotheticalBallPosition,
    behavior_tree::NodeTrace,
    field_dimensions::{FieldDimensions, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{BodyMotion, HeadMotion, MotionCommand},
    motion_type::MotionType,
    obstacles::Obstacle,
    parameters::BehaviorParameters,
    path_obstacles::PathObstacle,
    primary_state::PrimaryState,
    rule_obstacles::RuleObstacle,
    world_state::{BallState, WorldState},
};
use voronoi::VoronoiGrid;

use crate::tree::create_tree;

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

    let parameters = node.bind_parameter_as::<BehaviorParameters>("behavior_node")?;
    let field_dimensions_sub = node
        .subscriber::<FieldDimensions>("field_dimensions")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let player_number_sub = node
        .subscriber::<PlayerNumber>("player_number")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let fall_down_state_sub = node
        .subscriber::<FallDownState>("inputs/fall_down_state")?
        .build()
        .await?;
    let ball_sub = node.subscriber::<BallState>("ball_state")?.build().await?;
    let filtered_game_controller_state_sub = node
        .subscriber::<Option<FilteredGameControllerState>>("filtered_game_controller_state")?
        .build()
        .await?;
    let ground_to_field_sub = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")?
        .build()
        .await?;
    let hypothetical_ball_position_sub = node
        .subscriber::<Vec<HypotheticalBallPosition<Ground>>>("hypothetical_ball_positions")?
        .build()
        .await?;
    let obstacles_sub = node
        .subscriber::<Vec<Obstacle>>("obstacles")?
        .build()
        .await?;
    let position_of_interest_sub = node
        .subscriber::<Point2<Ground>>("position_of_interest")?
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
    let rule_ball_sub = node
        .subscriber::<BallState>("rule_ball_state")?
        .build()
        .await?;
    let rule_obstacles_sub = node
        .subscriber::<Vec<RuleObstacle>>("rule_obstacles")?
        .build()
        .await?;
    let suggested_search_position_sub = node
        .subscriber::<Point2<Field>>("suggested_search_position")?
        .build()
        .await?;
    let behavior_trace_pub = node
        .publisher::<NodeTrace>("behavior/trace")?
        .build()
        .await?;
    let behavior_tree_layout_pub = node
        .publisher::<NodeTrace>("behavior/tree_layout")?
        .build()
        .await?;
    let time_since_last_switch_pub = node
        .publisher::<Duration>("behavior/time_since_last_switch")?
        .build()
        .await?;
    let path_obstacles_output_pub = node
        .publisher::<Vec<PathObstacle>>("path_obstacles")?
        .build()
        .await?;
    let motion_command_pub = node
        .publisher::<MotionCommand>("motion_command")?
        .build()
        .await?;

    let tree = create_tree();
    let static_layout = tree.static_layout_trace();

    // let mut last_sent_game_controller_return_message_time = None;
    //let mut last_senthsl_message_time = None;

    let mut blackboard = Blackboard {
        field_dimensions: field_dimensions_sub.recv().await?,
        parameters: parameters.snapshot().typed().clone(),
        world_state: WorldState::default(),

        path_obstacles_output: Vec::new(),
        time_since_last_switch: Duration::ZERO,
        direction_difference: 0.0,
        voronoi_inputs: Vec::new(),

        ball: None,
        last_ball: None,
        last_close_enough_to_kick: false,
        last_motion_command: MotionCommand::default(),
        last_motion_switch_time: SystemTime::UNIX_EPOCH,
        last_motion_type: None,

        is_injected_motion_command: false,
        walk_position: None,
        body_motion: None,
        head_motion: None,
        voronoi_map: None,
    };

    loop {}
}
