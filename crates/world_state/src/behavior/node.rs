use std::net::SocketAddr;
use std::time::{Duration, SystemTime};

use color_eyre::Result;

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use hardware::NetworkInterface;
use hsl_network_messages::HulkMessage;
use linear_algebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};
use types::{
    behavior_tree::NodeTrace,
    field_dimensions::FieldDimensions,
    motion_command::{BodyMotion, HeadMotion, MotionCommand},
    motion_type::MotionType,
    parameters::{BehaviorParameters, HslNetworkParameters},
    path_obstacles::PathObstacle,
    world_state::WorldState,
};

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
    pub last_motion_switch_time: SystemTime,
    pub last_motion_type: Option<MotionType>,
    #[serde(skip, default = "create_tree_default")]
    pub tree: Node<Blackboard>,
    #[serde(skip, default = "create_static_layout_default")]
    pub static_layout: NodeTrace,
    pub last_sent_game_controller_return_message_time: Option<SystemTime>,
    pub last_sent_hsl_message_time: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastBall {
    pub position: Point2<Field>,
    pub velocity: Vector2<Ground>,
    pub age: SystemTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct Blackboard {
    pub world_state: WorldState,
    pub parameters: BehaviorParameters,
    pub field_dimensions: FieldDimensions,
    pub last_motion_command: MotionCommand,

    pub path_obstacles_output: Vec<PathObstacle>,
    pub time_since_last_switch: Duration,

    pub ball: Option<LastBall>,
    pub last_ball: Option<LastBall>,
    pub last_close_enough_to_kick: bool,
    pub last_motion_switch_time: SystemTime,
    pub last_motion_type: Option<MotionType>,

    pub is_injected_motion_command: bool,
    pub body_motion: Option<BodyMotion>,
    pub head_motion: Option<HeadMotion>,
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
    hsl_network_parameters: Parameter<HslNetworkParameters, "hsl_network">,
    parameters: Parameter<BehaviorParameters, "behavior">,

    behavior_trace: AdditionalOutput<NodeTrace, "behavior.trace">,
    behavior_tree_layout: AdditionalOutput<NodeTrace, "behavior.tree_layout">,
    last_sent_message: AdditionalOutput<HulkMessage, "last_sent_message">,
    path_obstacles_output: AdditionalOutput<Vec<PathObstacle>, "path_obstacles">,
    time_since_last_switch: AdditionalOutput<Duration, "behavior.time_since_last_switch">,

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
            last_motion_switch_time: SystemTime::UNIX_EPOCH,
            last_motion_type: None,
            tree,
            static_layout,
            last_sent_game_controller_return_message_time: None,
            last_sent_hsl_message_time: None,
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl NetworkInterface>,
    ) -> Result<MainOutputs> {
        context
            .behavior_tree_layout
            .fill_if_subscribed(|| self.static_layout.clone());

        if let Some(ball) = context.world_state.ball {
            self.ball = Some(LastBall {
                position: ball.ball_in_field,
                velocity: ball.ball_in_ground_velocity,
                age: context.world_state.now,
            });
            self.last_ball = self.ball.clone();
        } else if let Some(last_ball) = &self.ball
            && context
                .world_state
                .now
                .duration_since(last_ball.age)
                .unwrap_or(Duration::from_secs(0))
                >= context.parameters.last_ball_timeout
        {
            self.ball = None;
        }

        let mut blackboard = Blackboard {
            world_state: context.world_state.clone(),
            parameters: context.parameters.clone(),
            field_dimensions: *context.field_dimensions,
            last_motion_command: context.last_motion_command.clone(),

            path_obstacles_output: Vec::new(),
            time_since_last_switch: Duration::ZERO,

            ball: self.ball.clone(),
            last_ball: self.last_ball.clone(),
            last_close_enough_to_kick: self.last_close_enough_to_kick,
            last_motion_switch_time: self.last_motion_switch_time,
            last_motion_type: self.last_motion_type,

            is_injected_motion_command: false,
            body_motion: None,
            head_motion: None,
        };
        let (status, trace) = self.tree.tick_with_trace(&mut blackboard);

        let motion_command: MotionCommand = assemble_motion_command(&blackboard, status)?;

        self.last_close_enough_to_kick = blackboard.last_close_enough_to_kick;
        *context.last_motion_command = motion_command.clone();

        let motion_type = match motion_command.clone() {
            MotionCommand::VisualKick { .. } => Some(MotionType::Kick),
            MotionCommand::Walk { .. } => Some(MotionType::Walk),
            MotionCommand::Stand { .. } => Some(MotionType::Stand),
            MotionCommand::StandUp => Some(MotionType::StandUp),
            MotionCommand::Prepare => Some(MotionType::Prepare),
            _ => None,
        };

        self.send_game_controller_return_message(
            context.world_state,
            context.game_controller_address,
            context.hsl_network_parameters,
            context.hardware,
        )?;

        self.send_base_message(
            context.world_state,
            context.hsl_network_parameters,
            context.remaining_amount_of_messages,
            &mut context.last_sent_message,
            context.hardware,
        )?;

        if motion_type != self.last_motion_type {
            self.last_motion_switch_time = context.world_state.now;
            self.last_motion_type = motion_type;
        }

        context.behavior_trace.fill_if_subscribed(|| trace);
        let path_obstacles_output = blackboard.path_obstacles_output;
        context
            .path_obstacles_output
            .fill_if_subscribed(|| path_obstacles_output);
        context
            .time_since_last_switch
            .fill_if_subscribed(|| blackboard.time_since_last_switch);

        Ok(MainOutputs {
            motion_command: motion_command.into(),
        })
    }
}
