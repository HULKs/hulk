use std::{net::SocketAddr, pin::Pin, sync::Arc, time::Duration};

use booster::FallDownState;
use color_eyre::Result;

use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Point2, Pose2, Vector2};
use ros_z::{prelude::*, qos::QosDurability, time::Time};
use serde::{Deserialize, Serialize};
use types::{
    ball_position::HypotheticalBallPosition,
    behavior_tree::NodeTrace,
    field_dimensions::{FieldDimensions, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    messages::OutgoingMessage,
    motion_command::{BodyMotion, HeadMotion, MotionCommand},
    motion_type::MotionType,
    obstacles::Obstacle,
    parameters::BehaviorParameters,
    path_obstacles::PathObstacle,
    players::Players,
    primary_state::PrimaryState,
    rule_obstacles::RuleObstacle,
    time_wrapper::TimeWrapper,
    world_state::{BallState, PlayerState, RobotState, WorldState},
};
use voronoi::VoronoiGrid;

use crate::{motion_assembler::assemble_motion_command, tree::create_tree};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastBall {
    pub position: Point2<Field>,
    pub velocity: Vector2<Ground>,
    pub age: Time,
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
    pub last_kick_target: Option<Point2<Field>>,
    pub last_motion_command: MotionCommand,
    pub last_motion_switch_time: Time,
    pub last_motion_type: Option<MotionType>,
    pub last_sent_game_controller_return_message_time: Option<Time>,
    pub last_sent_hsl_message_time: Option<Time>,

    pub is_injected_motion_command: bool,
    pub walk_position: Option<Point2<Ground>>,
    pub body_motion: Option<BodyMotion>,
    pub head_motion: Option<HeadMotion>,
    pub voronoi_map: Option<VoronoiGrid>,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("behavior_node").build().await?;

    let parameters = node.bind_parameter_as::<BehaviorParameters>("behavior_node")?;
    let free_kick_obstacle_radius =
        node.bind_parameter_as::<f32>("rule_obstacles.free_kick_obstacle_radius")?;
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
    let player_states_cache = node
        .create_cache::<Players<Option<TimeWrapper<PlayerState>>>>("player_states", 1)?
        .build()
        .await?;
    let fall_down_state_cache = node
        .create_cache::<FallDownState>("inputs/fall_down_state", 1)?
        .build()
        .await?;
    let ball_cache = node
        .create_cache::<BallState>("ball_state", 1)?
        .build()
        .await?;
    let filtered_game_controller_state_cache = node
        .create_cache::<TimeWrapper<FilteredGameControllerState>>(
            "filtered_game_controller_state",
            1,
        )?
        .build()
        .await?;
    let game_controller_address_cache = node
        .create_cache::<Option<SocketAddr>>("game_controller_address", 1)?
        .build()
        .await?;
    let ground_to_field_cache = node
        .create_cache::<Isometry2<Ground, Field>>("ground_to_field", 1)?
        .build()
        .await?;
    let hypothetical_ball_positions_cache = node
        .create_cache::<Vec<HypotheticalBallPosition<Ground>>>("hypothetical_ball_positions", 1)?
        .build()
        .await?;
    let obstacles_cache = node
        .create_cache::<Vec<Obstacle>>("obstacles", 1)?
        .build()
        .await?;
    let position_of_interest_cache = node
        .create_cache::<Point2<Ground>>("position_of_interest", 1)?
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
    let rule_ball_cache = node
        .create_cache::<BallState>("rule_ball_state", 1)?
        .build()
        .await?;
    let rule_obstacles_cache = node
        .create_cache::<Vec<RuleObstacle>>("rule_obstacles", 1)?
        .build()
        .await?;
    let suggested_search_position_cache = node
        .create_cache::<Point2<Field>>("suggested_search_position", 1)?
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
    let outgoing_message_pub = node
        .publisher::<OutgoingMessage>("outputs/message")?
        .build()
        .await?;

    let tree = create_tree();
    let _static_layout = tree.static_layout_trace();
    let mut timer = node.create_timer(Duration::from_millis(10));

    let player_number = player_number_sub.recv().await?;
    let primary_state = primary_state_sub.recv().await?;

    let mut blackboard = Blackboard {
        field_dimensions: field_dimensions_sub.recv().await?,
        free_kick_obstacle_radius: *free_kick_obstacle_radius.snapshot().typed(),
        parameters: parameters.snapshot().typed().clone(),
        world_state: WorldState::default(),

        path_obstacles_output: Vec::new(),
        time_since_last_switch: Duration::ZERO,
        direction_difference: 0.0,
        voronoi_inputs: Vec::new(),

        ball: None,
        last_ball: None,
        last_close_enough_to_kick: false,
        last_kick_target: None,
        last_motion_command: MotionCommand::default(),
        last_motion_switch_time: Time::zero(),
        last_motion_type: None,
        last_sent_game_controller_return_message_time: None,
        last_sent_hsl_message_time: None,

        is_injected_motion_command: false,
        walk_position: None,
        body_motion: None,
        head_motion: None,
        voronoi_map: None,
    };

    loop {
        blackboard.parameters = parameters.snapshot().typed().clone();
        blackboard.free_kick_obstacle_radius = *free_kick_obstacle_radius.snapshot().typed();

        let player_states = player_states_cache
            .get_latest()
            .map(|player_states| {
                player_states
                    .as_ref()
                    .clone()
                    .map(|player_state| player_state.map(|state| state.inner))
            })
            .unwrap_or_default();

        blackboard.world_state.robot = RobotState {
            ground_to_field: ground_to_field_cache
                .get_latest()
                .map(|ground_to_field| *ground_to_field),
            player_number,
            primary_state,
        };

        blackboard.world_state.ball = ball_cache.get_latest().map(|ball| *ball);
        blackboard.world_state.fall_down_state = fall_down_state_cache
            .get_latest()
            .map(|fall_down_state| *fall_down_state.as_ref());
        blackboard.world_state.filtered_game_controller_state =
            filtered_game_controller_state_cache
                .get_latest()
                .map(|filtered_game_controller_state| filtered_game_controller_state.inner.clone());
        blackboard.world_state.hypothetical_ball_positions = hypothetical_ball_positions_cache
            .get_latest()
            .map(|positions| positions.as_ref().clone())
            .unwrap_or_default();
        blackboard.world_state.now = node.clock().now();
        blackboard.world_state.obstacles = obstacles_cache
            .get_latest()
            .map(|obstacles| obstacles.as_ref().clone())
            .unwrap_or_default();
        blackboard.world_state.player_states = player_states;
        blackboard.world_state.position_of_interest = position_of_interest_cache
            .get_latest()
            .map(|position| *position)
            .unwrap_or_default();
        blackboard.world_state.rule_ball = rule_ball_cache.get_latest().map(|ball| *ball);
        blackboard.world_state.rule_obstacles = rule_obstacles_cache
            .get_latest()
            .map(|obstacles| obstacles.as_ref().clone())
            .unwrap_or_default();
        blackboard.world_state.suggested_search_position = suggested_search_position_cache
            .get_latest()
            .map(|position| *position);

        if let Some(ball) = blackboard.world_state.ball {
            blackboard.ball = Some(LastBall {
                position: ball.ball_in_field,
                velocity: ball.ball_in_ground_velocity,
                age: blackboard.world_state.now,
                field_side: ball.field_side,
            });
            blackboard.last_ball.clone_from(&blackboard.ball);
        } else if let Some(last_ball) = &blackboard.ball
            && blackboard.world_state.now.duration_since(last_ball.age)
                >= blackboard.parameters.last_ball_timeout
        {
            blackboard.ball = None;
        }

        let (status, _trace) = tree.tick_with_trace(&mut blackboard);
        let motion_command: MotionCommand = assemble_motion_command(&blackboard, status)?;

        blackboard.last_motion_command = motion_command.clone();

        let motion_type = match motion_command.clone() {
            MotionCommand::VisualKick { .. } => Some(MotionType::Kick),
            MotionCommand::Walk { .. } => Some(MotionType::Walk),
            MotionCommand::Stand { .. } => Some(MotionType::Stand),
            MotionCommand::StandUp => Some(MotionType::StandUp),
            MotionCommand::Prepare => Some(MotionType::Prepare),
            _ => None,
        };

        if motion_type != blackboard.last_motion_type {
            blackboard.last_motion_switch_time = blackboard.world_state.now;
            blackboard.last_motion_type = motion_type;
        }

        let game_controller_address = game_controller_address_cache
            .get_latest()
            .and_then(|address| *address);
        if let Some(message) =
            blackboard.game_controller_return_message(game_controller_address.as_ref())
        {
            outgoing_message_pub.publish(&message).await?;
        }

        if let Some(message) = blackboard.state_message() {
            outgoing_message_pub.publish(&message).await?;
        }

        timer.tick().await;
    }
}
