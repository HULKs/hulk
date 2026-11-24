use std::{
    collections::HashMap,
    net::SocketAddr,
    num::NonZeroUsize,
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime},
};

use color_eyre::Result;
use types::fall_detection::FallDetection;

use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use kinematics::joints::head::HeadJoints;
use linear_algebra::{Isometry2, Point2, Pose2, Vector2};
use ros_z::{
    prelude::*,
    qos::{QosDurability, QosHistory},
    time::Time,
};
use serde::{Deserialize, Serialize};
use tokio::task::block_in_place;
use tracing::info;
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    behavior_tree::NodeTrace,
    controller_input::{Button, ControllerInput},
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
    walking_velocity_limits::{WALKING_VELOCITY_LIMITS_TOPIC, WalkingVelocityLimits},
    world_state::{BallSource, BallState, PlayerState, RobotState, WorldState},
};
use voronoi::VoronoiGrid;

use crate::{
    motion_assembler::assemble_motion_command,
    send_message::{recent_game_controller_address, state_message_send_interval},
    tree::create_tree,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct LastBall {
    pub position: Point2<Field>,
    pub velocity: Vector2<Field>,
    pub age: Time,
    pub field_side: Side,
    pub source: BallSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct Blackboard {
    pub field_dimensions: FieldDimensions,
    pub player_number: PlayerNumber,
    pub head_yaw: f32,
    pub parameters: BehaviorParameters,
    pub walking_velocity_limits: WalkingVelocityLimits,
    pub search_cell_size: Option<Vector2<Field>>,
    pub world_state: WorldState,
    pub controller_input: Option<ControllerInput>,
    pub remote_control_enabled: bool,

    pub path_obstacles_output: Vec<PathObstacle>,
    pub time_since_last_switch: Duration,
    pub direction_difference: f32,
    pub voronoi_inputs: Vec<Pose2<Field>>,

    pub ball: Option<LastBall>,
    pub visual_kick_ball_position: Option<BallPosition<Ground>>,
    pub last_ball: Option<LastBall>,
    pub last_close_enough_to_kick: bool,
    /// Target selected for this tick, used to choose kick strength.
    pub kick_target: Option<Point2<Ground>>,
    pub last_kick_target: Option<Point2<Field>>,
    pub last_motion_command: Option<MotionCommand>,
    pub last_motion_switch_time: Time,
    pub last_motion_type: Option<MotionType>,
    pub last_sent_game_controller_return_message_time: Option<Time>,
    pub last_sent_hsl_message_time: Option<Time>,
    pub last_closest_to_ball: bool,
    pub closest_to_ball_entered_area_since: Option<Time>,
    pub closest_to_ball_left_area_since: Option<Time>,

    pub is_injected_motion_command: bool,
    pub walk_position: Option<Point2<Ground>>,
    pub body_motion: Option<BodyMotion>,
    pub head_motion: Option<HeadMotion>,
    pub voronoi_map: Option<VoronoiGrid>,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub fn ball_timeout(parameters: &BehaviorParameters, source: BallSource) -> Duration {
    match source {
        BallSource::Own => parameters.ball.last_ball_timeout,
        BallSource::Team => parameters.ball.team_ball_timeout,
        BallSource::Rule => Duration::ZERO, // A rule hypothesis is not a remembered observation.
    }
}

fn validate_behavior_parameters(
    parameters: &BehaviorParameters,
) -> std::result::Result<(), String> {
    let walk_speed = &parameters.walking.speed;
    let mut errors = Vec::new();

    if !walk_speed.velocity_fade_distance.is_finite() || walk_speed.velocity_fade_distance <= 0.0 {
        errors.push(format!(
            "walking.speed.velocity_fade_distance must be finite and strictly positive (got {})",
            walk_speed.velocity_fade_distance
        ));
    }

    let minimum_speed_is_valid =
        walk_speed.minimum_speed.is_finite() && walk_speed.minimum_speed >= 0.0;
    if !minimum_speed_is_valid {
        errors.push(format!(
            "walking.speed.minimum_speed must be finite and non-negative (got {})",
            walk_speed.minimum_speed
        ));
    }

    for (field, speed) in [
        ("walking.speed.kicking", walk_speed.kicking),
        ("walking.speed.search", walk_speed.search),
        ("walking.speed.blocking", walk_speed.blocking),
    ] {
        if !speed.is_finite() {
            errors.push(format!("{field} must be finite (got {speed})"));
            continue;
        }
        if speed < 0.0 {
            errors.push(format!("{field} must be non-negative (got {speed})"));
        }
        if minimum_speed_is_valid && speed < walk_speed.minimum_speed {
            errors.push(format!(
                "{field} must be greater than or equal to walking.speed.minimum_speed (got {speed} < {})",
                walk_speed.minimum_speed
            ));
        }
    }

    if !parameters.kicking.target_speed.is_finite() || parameters.kicking.target_speed < 0.0 {
        errors.push("kicking.target_speed must be finite and non-negative".to_owned());
    }

    let search = &parameters.search;
    if !search.position_reached_cell_fraction.is_finite()
        || !(0.0..=1.0).contains(&search.position_reached_cell_fraction)
        || !search.minimum_position_reached_distance.is_finite()
        || search.minimum_position_reached_distance < 0.0
    {
        errors.push("search arrival fraction must be in [0, 1] and minimum distance non-negative, both finite".into());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("behavior_node").build().await?;

    let parameters = node.bind_parameter_as::<BehaviorParameters>("behavior_node")?;
    parameters.add_validation_hook(validate_behavior_parameters)?;
    let search_cell_size_cache = node
        .subscriber::<Vector2<Field>>(types::heatmap::SEARCH_CELL_SIZE_TOPIC)
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;
    let field_dimensions_cache = node
        .subscriber::<FieldDimensions>("field_dimensions")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;
    let walking_velocity_limits_cache = node
        .subscriber::<WalkingVelocityLimits>(WALKING_VELOCITY_LIMITS_TOPIC)
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;

    let player_number_cache = node
        .subscriber::<PlayerNumber>("player_number")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;
    let player_states_cache = node
        .subscriber::<Players<Option<TimeWrapper<PlayerState>>>>("player_states")
        .cache(1)
        .build()
        .await?;
    let fall_detection_cache = node
        .subscriber::<FallDetection>("fall_detection/status")
        .cache(1)
        .build()
        .await?;
    let controller_input_cache = node
        .subscriber::<ControllerInput>("inputs/controller_input")
        .cache(1)
        .build()
        .await?;
    let ball_state_cache = node
        .subscriber::<Option<BallState>>("ball_state")
        .cache(1)
        .build()
        .await?;
    let visual_kick_ball_position_cache = node
        .subscriber::<Option<BallPosition<Ground>>>("visual_kick/ball_position")
        .cache(1)
        .build()
        .await?;
    let filtered_game_controller_state_cache = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .cache(1)
        .build()
        .await?;
    let game_controller_contacts_cache = node
        .subscriber::<HashMap<SocketAddr, Time>>("game_controller_address_contacts_times")
        .cache(1)
        .build()
        .await?;
    let ground_to_field_cache = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")
        .cache(1)
        .build()
        .await?;
    let head_joints_command_cache = node
        .subscriber::<HeadJoints<f32>>("head_joints_command")
        .cache(1)
        .build()
        .await?;
    let hypothetical_ball_positions_cache = node
        .subscriber::<Vec<HypotheticalBallPosition<Ground>>>("hypothetical_ball_positions")
        .cache(1)
        .build()
        .await?;
    let obstacles_cache = node
        .subscriber::<Vec<Obstacle>>("obstacles")
        .cache(1)
        .build()
        .await?;
    let position_of_interest_cache = node
        .subscriber::<Point2<Ground>>("position_of_interest")
        .cache(1)
        .build()
        .await?;
    let primary_state_cache = node
        .subscriber::<PrimaryState>("primary_state")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;
    let rule_ball_cache = node
        .subscriber::<Option<BallState>>("rule_ball_state")
        .cache(1)
        .build()
        .await?;
    let rule_obstacles_cache = node
        .subscriber::<Vec<RuleObstacle>>("rule_obstacles")
        .cache(1)
        .build()
        .await?;
    let suggested_search_position_cache = node
        .subscriber::<Option<Point2<Field>>>("suggested_search_position")
        .cache(1)
        .build()
        .await?;
    let additional_behavior_trace_pub = node
        .publisher::<NodeTrace>("behavior/trace")
        .build()
        .await?;
    let additional_behavior_tree_layout_pub = node
        .publisher::<NodeTrace>("behavior/tree_layout")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let additional_black_board_pub = node
        .publisher::<Blackboard>("behavior/blackboard")
        .build()
        .await?;
    let outgoing_message_pub = node
        .publisher::<OutgoingMessage>("outputs/message")
        .build()
        .await?;
    let hsl_state_message_send_interval_pub = node
        .publisher::<Option<Duration>>("hsl_state_message_send_interval")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            history: QosHistory::KeepLast(NonZeroUsize::MIN),
            ..Default::default()
        })
        .build()
        .await?;
    let goalkeeper_player_number_pub = node
        .publisher::<PlayerNumber>("behavior/goalkeeper_player_number")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            history: QosHistory::KeepLast(NonZeroUsize::MIN),
            ..Default::default()
        })
        .build()
        .await?;
    let motion_command_pub = node
        .publisher::<MotionCommand>("behavior/motion_command")
        .build()
        .await?;

    let tree = create_tree();
    let static_layout = tree.static_layout_trace();
    additional_behavior_tree_layout_pub
        .publish_if_subscribed(|| async { static_layout })
        .await?;
    let mut timer = node.create_timer(Duration::from_millis(20));
    let (field_dimensions, player_number) = loop {
        if let (Some(field_dimensions), Some(player_number)) = (
            field_dimensions_cache.get_latest(),
            player_number_cache.get_latest(),
        ) {
            break (*field_dimensions, *player_number);
        }
        motion_command_pub.publish(&MotionCommand::Damping).await?;
        timer.tick().await;
    };

    let mut blackboard = Blackboard {
        field_dimensions,
        player_number,
        head_yaw: 0.0,
        parameters: parameters.snapshot().typed().clone(),
        walking_velocity_limits: walking_velocity_limits_cache
            .get_latest()
            .and_then(|limits| limits.validate().is_ok().then_some(*limits))
            .unwrap_or_default(),
        search_cell_size: search_cell_size_cache.get_latest().map(|size| *size),
        world_state: WorldState::default(),
        controller_input: None,
        remote_control_enabled: false,

        path_obstacles_output: Vec::new(),
        time_since_last_switch: Duration::ZERO,
        direction_difference: 0.0,
        voronoi_inputs: Vec::new(),

        ball: None,
        visual_kick_ball_position: None,
        last_ball: None,
        last_close_enough_to_kick: false,
        kick_target: None,
        last_kick_target: None,
        last_motion_command: None,
        last_motion_switch_time: Time::zero(),
        last_motion_type: None,
        last_sent_game_controller_return_message_time: None,
        last_sent_hsl_message_time: None,
        last_closest_to_ball: false,
        closest_to_ball_entered_area_since: None,
        closest_to_ball_left_area_since: None,

        is_injected_motion_command: false,
        walk_position: None,
        body_motion: None,
        head_motion: None,
        voronoi_map: None,
    };

    let mut published_state_message_send_interval = None;
    hsl_state_message_send_interval_pub
        .publish(&published_state_message_send_interval)
        .await?;
    let mut published_goalkeeper_player_number = blackboard.parameters.goalkeeper.player_number;
    goalkeeper_player_number_pub
        .publish(&published_goalkeeper_player_number)
        .await?;

    loop {
        timer.tick().await;

        if let Some(field_dimensions) = field_dimensions_cache.get_latest() {
            blackboard.field_dimensions = *field_dimensions;
        }

        blackboard.path_obstacles_output.clear();
        blackboard.time_since_last_switch = Duration::ZERO;
        blackboard.direction_difference = 0.0;
        blackboard.voronoi_inputs.clear();

        blackboard.is_injected_motion_command = false;
        blackboard.walk_position = None;
        blackboard.kick_target = None;
        blackboard.body_motion = None;
        blackboard.head_motion = None;
        blackboard.voronoi_map = None;

        if let Some(player_number) = player_number_cache.get_latest() {
            blackboard.player_number = *player_number;
        }
        blackboard.head_yaw = head_joints_command_cache
            .get_latest()
            .map(|head_joints| head_joints.yaw)
            .unwrap_or_default();
        blackboard.parameters = parameters.snapshot().typed().clone();
        if let Some(limits) = walking_velocity_limits_cache.get_latest()
            && limits.validate().is_ok()
        {
            blackboard.walking_velocity_limits = *limits;
        }
        blackboard.search_cell_size = search_cell_size_cache.get_latest().map(|size| *size);
        let goalkeeper_player_number = blackboard.parameters.goalkeeper.player_number;
        if goalkeeper_player_number != published_goalkeeper_player_number {
            goalkeeper_player_number_pub
                .publish(&goalkeeper_player_number)
                .await?;
            published_goalkeeper_player_number = goalkeeper_player_number;
        }

        let was_start_pressed = blackboard
            .controller_input
            .as_ref()
            .is_some_and(|input| input.is_pressed(Button::Start));
        blackboard.controller_input = controller_input_cache
            .get_after(Time::from_wallclock(SystemTime::now()) - Duration::from_millis(250))
            .filter(|input| input.connected)
            .map(|input| input.as_ref().clone());
        if let Some(input) = &blackboard.controller_input
            && input.is_pressed(Button::Start)
            && !was_start_pressed
        {
            blackboard.remote_control_enabled = !blackboard.remote_control_enabled;
        }

        let player_states = player_states_cache
            .get_latest()
            .map(|player_states| {
                player_states
                    .as_ref()
                    .clone()
                    .map(|player_state| player_state.map(|state| state.inner))
            })
            .unwrap_or_default();

        let primary_state = primary_state_cache
            .get_latest()
            .map(|state| *state)
            .unwrap_or(PrimaryState::Damping);
        if primary_state != blackboard.world_state.robot.primary_state {
            blackboard.remote_control_enabled = false;
        }

        blackboard.world_state.robot = RobotState {
            ground_to_field: ground_to_field_cache
                .get_latest()
                .map(|ground_to_field| *ground_to_field),
            primary_state,
        };

        blackboard.world_state.ball = ball_state_cache.get_latest().and_then(|ball| *ball);
        blackboard.visual_kick_ball_position = visual_kick_ball_position_cache
            .get_latest()
            .and_then(|ball_position| *ball_position);
        blackboard.world_state.fall_detection = fall_detection_cache
            .get_latest()
            .map(|fall_detection| *fall_detection.as_ref());
        blackboard.world_state.filtered_game_controller_state =
            filtered_game_controller_state_cache.get_latest().map(
                |filtered_game_controller_state| filtered_game_controller_state.as_ref().clone(),
            );
        let send_interval = blackboard
            .world_state
            .filtered_game_controller_state
            .as_ref()
            .and_then(|state| state_message_send_interval(state, &blackboard.parameters.network));
        if send_interval != published_state_message_send_interval {
            hsl_state_message_send_interval_pub
                .publish(&send_interval)
                .await?;
            published_state_message_send_interval = send_interval;
        }
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
        blackboard.world_state.rule_ball = rule_ball_cache.get_latest().and_then(|ball| *ball);
        blackboard.world_state.rule_obstacles = rule_obstacles_cache
            .get_latest()
            .map(|obstacles| obstacles.as_ref().clone())
            .unwrap_or_default();
        blackboard.world_state.suggested_search_position = suggested_search_position_cache
            .get_latest()
            .and_then(|position| *position);

        if let (Some(ball), Some(ground_to_field)) = (
            blackboard.world_state.ball,
            blackboard.world_state.robot.ground_to_field,
        ) {
            blackboard.ball = Some(LastBall {
                position: ball.ball_in_field,
                velocity: ground_to_field * ball.ball_in_ground_velocity,
                age: blackboard.world_state.now,
                field_side: ball.field_side,
                source: ball.source,
            });
            blackboard.last_ball.clone_from(&blackboard.ball);
        } else if let Some(last_ball) = &blackboard.ball
            && blackboard.world_state.now.duration_since(last_ball.age)
                >= ball_timeout(&blackboard.parameters, last_ball.source)
        {
            blackboard.ball = None;
        }

        let record_trace = additional_behavior_trace_pub.has_subscribers();
        let (status, trace) = block_in_place(|| tree.tick(&mut blackboard, record_trace));
        let motion_command: MotionCommand = assemble_motion_command(&blackboard, status)?;

        let previous_motion_command = blackboard.last_motion_command.clone();
        blackboard.last_motion_command = Some(motion_command.clone());

        let motion_type = match &motion_command {
            MotionCommand::Damping => Some(MotionType::Damping),
            MotionCommand::Kick { .. } => Some(MotionType::Kick),
            MotionCommand::Walk { .. } | MotionCommand::WalkWithVelocity { .. } => {
                Some(MotionType::Walk)
            }
            MotionCommand::Stand { .. } => Some(MotionType::Stand),
            MotionCommand::StandUp => Some(MotionType::StandUp),
            MotionCommand::Prepare => Some(MotionType::Prepare),
        };

        if previous_motion_command.as_ref() != Some(&motion_command) {
            info!(
                target: "behavior_node::motion",
                ?motion_command,
                ?motion_type,
                previous_motion_type = ?blackboard.last_motion_type,
                "motion command changed"
            );
        }

        if motion_type != blackboard.last_motion_type {
            blackboard.last_motion_switch_time = blackboard.world_state.now;
            blackboard.last_motion_type = motion_type;
        }

        let game_controller_address =
            game_controller_contacts_cache
                .get_latest()
                .and_then(|contacts| {
                    recent_game_controller_address(
                        &contacts,
                        blackboard.world_state.now,
                        blackboard
                            .parameters
                            .network
                            .game_controller_message_receive_timeout,
                    )
                });
        if let Some(message) =
            blackboard.game_controller_return_message(game_controller_address.as_ref())
        {
            outgoing_message_pub.publish(&message).await?;
        }

        if let Some(message) = blackboard.try_sending_state_message() {
            outgoing_message_pub.publish(&message).await?;
        }

        if let Some(trace) = trace {
            additional_behavior_trace_pub.publish(&trace).await?;
        }
        additional_black_board_pub
            .publish_if_subscribed(|| async { &blackboard })
            .await?;
        motion_command_pub.publish(&motion_command).await?;
    }
}

#[cfg(test)]
mod runtime_tests;
