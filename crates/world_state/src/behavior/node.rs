use std::net::SocketAddr;
use std::time::{Duration, SystemTime};

use color_eyre::Result;

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use hardware::NetworkInterface;
use hsl_network_messages::HulkMessage;
use linear_algebra::{Point2, Pose2, Vector2};
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

use crate::behavior::{
    behavior_tree::Node, motion_assembler::assemble_motion_command, tree::create_tree,
};

fn create_tree_default() -> Node<Blackboard> {
    create_tree()
}

fn create_static_layout_default() -> NodeTrace {
    create_tree().static_layout_trace()
}

#[derive(Serialize, Deserialize)]
pub struct Behavior {
    pub ball: Option<LastBall>,
    pub last_ball: Option<LastBall>,
    pub last_close_enough_to_kick: bool,
    pub last_kick_target: Option<Point2<Field>>,
    pub last_motion_switch_time: SystemTime,
    pub last_motion_type: Option<MotionType>,
    #[serde(skip, default = "create_tree_default")]
    pub tree: Node<Blackboard>,
    #[serde(skip, default = "create_static_layout_default")]
    pub static_layout: NodeTrace,
    pub last_sent_game_controller_return_message_time: Option<SystemTime>,
    pub last_sent_hsl_message_time: Option<SystemTime>,
    pub last_closest_to_ball: bool,
    pub closest_to_ball_entered_area_since: Option<SystemTime>,
    pub closest_to_ball_left_area_since: Option<SystemTime>,
}

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
    pub last_kick_target: Option<Point2<Field>>,
    pub last_motion_command: MotionCommand,
    pub last_motion_switch_time: SystemTime,
    pub last_motion_type: Option<MotionType>,
    pub last_closest_to_ball: bool,
    pub closest_to_ball_entered_area_since: Option<SystemTime>,
    pub closest_to_ball_left_area_since: Option<SystemTime>,

    pub is_injected_motion_command: bool,
    pub walk_position: Option<Point2<Ground>>,
    pub body_motion: Option<BodyMotion>,
    pub head_motion: Option<HeadMotion>,
    pub voronoi_map: Option<VoronoiGrid>,
}

pub struct BehaviorTickInput {
    pub world_state: WorldState,
    pub field_dimensions: FieldDimensions,
    pub parameters: BehaviorParameters,
    pub free_kick_obstacle_radius: f32,
    pub last_motion_command: MotionCommand,
}

pub struct BehaviorTickOutput {
    pub motion_command: MotionCommand,
    pub trace: NodeTrace,
    pub static_layout: NodeTrace,
    pub path_obstacles: Vec<PathObstacle>,
    pub time_since_last_switch: Duration,
    pub direction_difference: f32,
    pub walk_position: Option<Point2<Ground>>,
    pub voronoi_map: Option<VoronoiGrid>,
    pub voronoi_inputs: Vec<Pose2<Field>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    game_controller_address: Input<Option<SocketAddr>, "game_controller_address?">,
    remaining_amount_of_messages:
        Input<Option<u16>, "game_controller_state?.hulks_team.remaining_amount_of_messages">,
    world_state: Input<WorldState, "world_state">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    parameters: Parameter<BehaviorParameters, "behavior">,
    free_kick_obstacle_radius: Parameter<f32, "rule_obstacles.free_kick_obstacle_radius">,

    behavior_trace: AdditionalOutput<NodeTrace, "behavior.trace">,
    behavior_tree_layout: AdditionalOutput<NodeTrace, "behavior.tree_layout">,
    last_sent_message: AdditionalOutput<HulkMessage, "last_sent_message">,
    path_obstacles_output: AdditionalOutput<Vec<PathObstacle>, "path_obstacles">,
    time_since_last_switch: AdditionalOutput<Duration, "behavior.time_since_last_switch">,
    direction_difference: AdditionalOutput<f32, "behavior.direction_difference">,
    walk_position: AdditionalOutput<Option<Point2<Ground>>, "behavior.walk_position">,
    voronoi_map: AdditionalOutput<Option<VoronoiGrid>, "behavior.voronoi_map">,
    voronoi_inputs: AdditionalOutput<Vec<Pose2<Field>>, "behavior.voronoi_inputs">,

    last_motion_command: CyclerState<MotionCommand, "last_motion_command">,

    hardware: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motion_command: MainOutput<MotionCommand>,
}

impl Behavior {
    pub fn new(_context: CreationContext) -> Result<Self> {
        let tree = create_tree();
        let static_layout = tree.static_layout_trace();

        Ok(Self {
            ball: None,
            last_ball: None,
            last_close_enough_to_kick: false,
            last_kick_target: None,
            last_motion_switch_time: SystemTime::UNIX_EPOCH,
            last_motion_type: None,
            tree,
            static_layout,
            last_sent_game_controller_return_message_time: None,
            last_sent_hsl_message_time: None,
            last_closest_to_ball: false,
            closest_to_ball_entered_area_since: None,
            closest_to_ball_left_area_since: None,
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl NetworkInterface>,
    ) -> Result<MainOutputs> {
        context
            .behavior_tree_layout
            .fill_if_subscribed(|| self.static_layout.clone());

        let tick_output = self.tick_behavior_tree(BehaviorTickInput {
            world_state: context.world_state.clone(),
            field_dimensions: *context.field_dimensions,
            parameters: context.parameters.clone(),
            free_kick_obstacle_radius: *context.free_kick_obstacle_radius,
            last_motion_command: context.last_motion_command.clone(),
        })?;

        let communication_output =
            self.plan_communication(crate::behavior::send_message::CommunicationInput {
                world_state: context.world_state,
                game_controller_address: context.game_controller_address.copied(),
                hsl_network_parameters: &context.parameters.hsl_network,
                remaining_amount_of_messages: context.remaining_amount_of_messages.copied(),
            });

        for message in communication_output.outgoing_messages {
            context.hardware.write_to_network(message)?;
        }

        if let Some(message) = communication_output.last_sent_message {
            context.last_sent_message.fill_if_subscribed(|| message);
        }

        *context.last_motion_command = tick_output.motion_command.clone();

        context
            .behavior_trace
            .fill_if_subscribed(|| tick_output.trace);
        context
            .path_obstacles_output
            .fill_if_subscribed(|| tick_output.path_obstacles);
        context
            .time_since_last_switch
            .fill_if_subscribed(|| tick_output.time_since_last_switch);
        context
            .direction_difference
            .fill_if_subscribed(|| tick_output.direction_difference);
        context
            .walk_position
            .fill_if_subscribed(|| tick_output.walk_position);
        context
            .voronoi_map
            .fill_if_subscribed(|| tick_output.voronoi_map);
        context
            .voronoi_inputs
            .fill_if_subscribed(|| tick_output.voronoi_inputs);

        Ok(MainOutputs {
            motion_command: tick_output.motion_command.into(),
        })
    }

    pub fn tick_behavior_tree(&mut self, input: BehaviorTickInput) -> Result<BehaviorTickOutput> {
        if let Some(ball) = input.world_state.ball {
            self.ball = Some(LastBall {
                position: ball.ball_in_field,
                velocity: ball.ball_in_ground_velocity,
                age: input.world_state.now.to_wallclock(),
                field_side: ball.field_side,
            });
            self.last_ball = self.ball.clone();
        } else if let Some(last_ball) = &self.ball
            && input
                .world_state
                .now
                .to_wallclock()
                .duration_since(last_ball.age)
                .unwrap_or(Duration::from_secs(0))
                >= input.parameters.last_ball_timeout
        {
            self.ball = None;
        }

        let mut blackboard = Blackboard {
            field_dimensions: input.field_dimensions,
            free_kick_obstacle_radius: input.free_kick_obstacle_radius,
            parameters: input.parameters.clone(),
            world_state: input.world_state.clone(),

            path_obstacles_output: Vec::new(),
            time_since_last_switch: Duration::ZERO,
            direction_difference: 0.0,
            voronoi_inputs: Vec::new(),

            ball: self.ball.clone(),
            last_ball: self.last_ball.clone(),
            last_close_enough_to_kick: self.last_close_enough_to_kick,
            last_kick_target: self.last_kick_target,
            last_motion_command: input.last_motion_command,
            last_motion_switch_time: self.last_motion_switch_time,
            last_motion_type: self.last_motion_type,
            last_closest_to_ball: self.last_closest_to_ball,
            closest_to_ball_entered_area_since: self.closest_to_ball_entered_area_since,
            closest_to_ball_left_area_since: self.closest_to_ball_left_area_since,

            is_injected_motion_command: false,
            walk_position: None,
            body_motion: None,
            head_motion: None,
            voronoi_map: None,
        };
        let (status, trace) = self.tree.tick_with_trace(&mut blackboard);

        let motion_command: MotionCommand = assemble_motion_command(&blackboard, status)?;
        self.last_kick_target = blackboard.last_kick_target;

        self.last_close_enough_to_kick = blackboard.last_close_enough_to_kick;
        self.last_closest_to_ball = blackboard.last_closest_to_ball;
        self.closest_to_ball_entered_area_since = blackboard.closest_to_ball_entered_area_since;
        self.closest_to_ball_left_area_since = blackboard.closest_to_ball_left_area_since;

        let motion_type = match motion_command.clone() {
            MotionCommand::VisualKick { .. } => Some(MotionType::Kick),
            MotionCommand::Walk { .. } => Some(MotionType::Walk),
            MotionCommand::Stand { .. } => Some(MotionType::Stand),
            MotionCommand::StandUp => Some(MotionType::StandUp),
            MotionCommand::Prepare => Some(MotionType::Prepare),
            _ => None,
        };

        if motion_type != self.last_motion_type {
            self.last_motion_switch_time = input.world_state.now.to_wallclock();
            self.last_motion_type = motion_type;
        }

        Ok(BehaviorTickOutput {
            motion_command,
            trace,
            static_layout: self.static_layout.clone(),
            path_obstacles: blackboard.path_obstacles_output,
            time_since_last_switch: blackboard.time_since_last_switch,
            direction_difference: blackboard.direction_difference,
            walk_position: blackboard.walk_position,
            voronoi_map: blackboard.voronoi_map,
            voronoi_inputs: blackboard.voronoi_inputs,
        })
    }
}
