use std::{net::SocketAddr, time::Duration};

use behavior_node::{
    behavior_tree::Node as BehaviorNodeTree, motion_assembler::assemble_motion_command,
    node::Blackboard as BehaviorBlackboard, tree::create_tree as create_behavior_tree,
};
use bevy::prelude::*;
use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use linear_algebra::{Point2, Pose2};
use types::{
    behavior_tree::NodeTrace,
    field_dimensions::FieldDimensions,
    messages::OutgoingMessage,
    motion_command::MotionCommand,
    parameters::{BehaviorParameters, HslNetworkParameters},
    path_obstacles::PathObstacle,
    world_state::WorldState,
};
use voronoi::VoronoiGrid;

use crate::behavior_tree_simulator::{
    RobotFrame, SimulatorFieldDimensions, SimulatorRobot, SimulatorRobotFrames,
    SimulatorRobotParameters, SimulatorWorldStates,
};

#[derive(Component)]
pub struct SimulatorRobotBehavior {
    pub tree: BehaviorNodeTree<BehaviorBlackboard>,
    pub blackboard: BehaviorBlackboard,
    pub static_layout: NodeTrace,
}

impl SimulatorRobotBehavior {
    pub fn new(parameters: BehaviorParameters) -> Self {
        let tree = create_behavior_tree();
        let static_layout = tree.static_layout_trace();
        Self {
            tree,
            blackboard: create_behavior_blackboard(parameters),
            static_layout,
        }
    }

    pub fn tick_behavior_tree(
        &mut self,
        input: SimulatorBehaviorTickInput,
    ) -> Result<SimulatorBehaviorTickOutput> {
        self.blackboard.field_dimensions = input.field_dimensions;
        self.blackboard.parameters = input.parameters;
        self.blackboard.world_state = input.world_state.clone();

        self.blackboard.path_obstacles_output.clear();
        self.blackboard.time_since_last_switch = Duration::ZERO;
        self.blackboard.direction_difference = 0.0;
        self.blackboard.voronoi_inputs.clear();
        self.blackboard.is_injected_motion_command = false;
        self.blackboard.walk_position = None;
        self.blackboard.body_motion = None;
        self.blackboard.head_motion = None;
        self.blackboard.voronoi_map = None;

        if let Some(ball) = self.blackboard.world_state.ball {
            self.blackboard.ball = Some(behavior_node::node::LastBall {
                position: ball.ball_in_field,
                velocity: ball.ball_in_ground_velocity,
                age: self.blackboard.world_state.now,
                field_side: ball.field_side,
            });
            self.blackboard.last_ball.clone_from(&self.blackboard.ball);
        } else if let Some(last_ball) = &self.blackboard.ball
            && self
                .blackboard
                .world_state
                .now
                .duration_since(last_ball.age)
                >= self.blackboard.parameters.last_ball_timeout
        {
            self.blackboard.ball = None;
        }

        let (status, trace) = self.tree.tick_with_trace(&mut self.blackboard);
        let motion_command = assemble_motion_command(&self.blackboard, status)?;
        self.blackboard.last_motion_command = motion_command.clone();

        let motion_type = match motion_command.clone() {
            MotionCommand::VisualKick { .. } => Some(types::motion_type::MotionType::Kick),
            MotionCommand::Walk { .. } => Some(types::motion_type::MotionType::Walk),
            MotionCommand::Stand { .. } => Some(types::motion_type::MotionType::Stand),
            MotionCommand::StandUp => Some(types::motion_type::MotionType::StandUp),
            MotionCommand::Prepare => Some(types::motion_type::MotionType::Prepare),
            _ => None,
        };

        if motion_type != self.blackboard.last_motion_type {
            self.blackboard.last_motion_switch_time = self.blackboard.world_state.now;
            self.blackboard.last_motion_type = motion_type;
        }

        Ok(SimulatorBehaviorTickOutput {
            motion_command,
            trace,
            static_layout: self.static_layout.clone(),
            path_obstacles: self.blackboard.path_obstacles_output.clone(),
            time_since_last_switch: self.blackboard.time_since_last_switch,
            direction_difference: self.blackboard.direction_difference,
            walk_position: self.blackboard.walk_position,
            voronoi_map: self.blackboard.voronoi_map.clone(),
            voronoi_inputs: self.blackboard.voronoi_inputs.clone(),
        })
    }

    pub fn plan_communication(
        &mut self,
        world_state: WorldState,
        hsl_network_parameters: HslNetworkParameters,
        game_controller_address: Option<SocketAddr>,
    ) -> Vec<OutgoingMessage> {
        self.blackboard.world_state = world_state;
        self.blackboard.parameters.hsl_network = hsl_network_parameters;

        let mut outgoing_messages = Vec::new();
        if let Some(message) = self
            .blackboard
            .game_controller_return_message(game_controller_address.as_ref())
        {
            outgoing_messages.push(message);
        }
        if let Some(message) = self.blackboard.state_message() {
            outgoing_messages.push(message);
        }
        outgoing_messages
    }
}

pub struct SimulatorBehaviorTickInput {
    pub world_state: WorldState,
    pub field_dimensions: FieldDimensions,
    pub parameters: BehaviorParameters,
}

pub struct SimulatorBehaviorTickOutput {
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

fn create_behavior_blackboard(parameters: BehaviorParameters) -> BehaviorBlackboard {
    BehaviorBlackboard {
        field_dimensions: FieldDimensions::default(),
        parameters,
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
        last_motion_switch_time: ros_z::time::Time::zero(),
        last_motion_type: None,
        last_sent_game_controller_return_message_time: None,
        last_sent_hsl_message_time: None,
        is_injected_motion_command: false,
        walk_position: None,
        body_motion: None,
        head_motion: None,
        voronoi_map: None,
    }
}

pub(crate) fn tick_behavior_trees(
    field_dimensions: Res<SimulatorFieldDimensions>,
    world_states: Res<SimulatorWorldStates>,
    mut robot_frames: ResMut<SimulatorRobotFrames>,
    mut robots: Query<(
        &SimulatorRobot,
        &SimulatorRobotParameters,
        &mut SimulatorRobotBehavior,
    )>,
) {
    robot_frames.0.clear();

    for (robot, parameters, mut behavior) in &mut robots {
        let Some(world_state) = world_states.0.get(&robot.player_number).cloned() else {
            continue;
        };

        let tick_output = behavior
            .tick_behavior_tree(SimulatorBehaviorTickInput {
                world_state: world_state.clone(),
                field_dimensions: field_dimensions.0,
                parameters: parameters.behavior.clone(),
            })
            .expect("behavior tree tick should not fail in simulator");
        robot_frames.0.insert(
            robot.player_number,
            RobotFrame::from_outputs(world_state, tick_output, Vec::new()),
        );
    }
}
