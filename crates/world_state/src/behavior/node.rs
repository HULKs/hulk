use std::time::{Duration, SystemTime};

use color_eyre::{Result, eyre::Error};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use linear_algebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};
use types::{
    behavior_tree::{NodeTrace, Status},
    field_dimensions::FieldDimensions,
    motion_command::{HeadMotion, ImageRegion, KickPower, MotionCommand},
    motion_type::MotionType,
    parameters::BehaviorParameters,
    path_obstacles::PathObstacle,
    world_state::WorldState,
};

use crate::behavior::{behavior_tree::Node, kick_selector::KickTarget, tree::create_tree};

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
    pub last_kick_power: Option<KickPower>,
    pub last_motion_switch_time: SystemTime,
    pub last_motion_type: Option<MotionType>,
    #[serde(skip, default = "create_tree_default")]
    pub tree: Node<Blackboard>,
    #[serde(skip, default = "create_static_layout_default")]
    pub static_layout: NodeTrace,
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

    pub is_alternative_kick: bool,
    pub path_obstacles_output: Vec<PathObstacle>,

    pub ball: Option<LastBall>,
    pub last_ball: Option<LastBall>,
    pub last_close_enough_to_kick: bool,
    pub last_kick_power: Option<KickPower>,
    pub last_motion_switch_time: SystemTime,
    pub last_motion_type: Option<MotionType>,
    pub time_since_last_switch: Duration,

    pub kick_target: Option<KickTarget>,
    pub motion: Option<MotionCommand>,
    pub head_motion: Option<HeadMotion>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    world_state: Input<WorldState, "world_state">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    parameters: Parameter<BehaviorParameters, "behavior">,

    behavior_trace: AdditionalOutput<NodeTrace, "behavior.trace">,
    behavior_tree_layout: AdditionalOutput<NodeTrace, "behavior.tree_layout">,
    is_alternative_kick: AdditionalOutput<bool, "behavior.is_alternative_kick">,
    time_since_last_switch: AdditionalOutput<Duration, "behavior.time_since_last_switch">,
    kick_target_distance: AdditionalOutput<Option<f32>, "behavior.kick_target_distance">,

    path_obstacles_output: AdditionalOutput<Vec<PathObstacle>, "path_obstacles">,

    last_motion_command: CyclerState<MotionCommand, "last_motion_command">,
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
            last_kick_power: None,
            last_motion_switch_time: SystemTime::UNIX_EPOCH,
            last_motion_type: None,
            tree,
            static_layout,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
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
            is_alternative_kick: false,
            path_obstacles_output: Vec::new(),
            ball: self.ball.clone(),
            last_ball: self.last_ball.clone(),
            last_close_enough_to_kick: self.last_close_enough_to_kick,
            last_kick_power: self.last_kick_power,
            last_motion_switch_time: self.last_motion_switch_time,
            last_motion_type: self.last_motion_type,
            time_since_last_switch: Duration::ZERO,
            motion: None,
            head_motion: None,
            kick_target: None,
        };
        let (status, trace) = self.tree.tick_with_trace(&mut blackboard);
        context.behavior_trace.fill_if_subscribed(|| trace);
        context
            .path_obstacles_output
            .fill_if_subscribed(|| blackboard.path_obstacles_output);
        context
            .time_since_last_switch
            .fill_if_subscribed(|| blackboard.time_since_last_switch);
        context
            .is_alternative_kick
            .fill_if_subscribed(|| blackboard.is_alternative_kick);
        context.kick_target_distance.fill_if_subscribed(|| {
            blackboard
                .kick_target
                .as_ref()
                .map(|kick_target| kick_target.position.coords().norm())
        });

        let motion_command: MotionCommand = match status {
            Status::Success => blackboard.motion.take().unwrap_or(MotionCommand::Stand {
                head: HeadMotion::Center {
                    image_region_target: ImageRegion::Center,
                },
            }),
            Status::Failure => MotionCommand::Stand {
                head: HeadMotion::Center {
                    image_region_target: ImageRegion::Center,
                },
            },
            Status::Idle => {
                return Err(Error::msg(
                    "Behavior tree returned Idle status, which should not happen during a cycle",
                ));
            }
        };

        self.last_close_enough_to_kick = blackboard.last_close_enough_to_kick;
        self.last_kick_power = blackboard.last_kick_power;
        *context.last_motion_command = motion_command.clone();

        let motion_type = match motion_command.clone() {
            MotionCommand::VisualKick { .. } => Some(MotionType::Kick),
            MotionCommand::Walk { .. } => Some(MotionType::Walk),
            MotionCommand::Stand { .. } => Some(MotionType::Stand),
            MotionCommand::StandUp => Some(MotionType::StandUp),
            MotionCommand::Prepare => Some(MotionType::Prepare),
            _ => None,
        };

        if motion_type != Some(MotionType::Kick) {
            self.last_kick_power = None;
        }

        if motion_type != self.last_motion_type {
            self.last_motion_switch_time = context.world_state.now;
            self.last_motion_type = motion_type;
        }

        Ok(MainOutputs {
            motion_command: motion_command.into(),
        })
    }
}
