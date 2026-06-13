use std::{collections::BTreeMap, time::Duration, time::SystemTime};

use bevy::prelude::*;
use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Point2, Pose2};
use serde::Serialize;
use types::{
    behavior_tree::NodeTrace, messages::OutgoingMessage, motion_command::MotionCommand,
    path_obstacles::PathObstacle, players::Players, world_state::WorldState,
};
use voronoi::VoronoiGrid;

use crate::behavior_tree_simulator::{
    InvariantViolation, RobotSnapshot, SimulatedBall, SimulatedRobot, SimulatorBall,
    SimulatorBehaviorTickOutput, SimulatorClock, SimulatorCurrentInvariantViolations,
    SimulatorFallDownState, SimulatorGroundToWorld, SimulatorPrimaryState, SimulatorRobot,
};

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorTimeline {
    pub frames: Vec<TimelineFrame>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorScenarioResult {
    pub failed: bool,
    pub failures: Vec<SimulatorFailure>,
}

#[derive(Clone, Debug, Serialize)]
pub enum SimulatorFailure {
    InvariantViolation(InvariantViolation),
    ScenarioAssertion(String),
}

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorRobotFrames(pub BTreeMap<PlayerNumber, RobotFrame>);

#[derive(Clone, Debug, Serialize)]
pub struct TimelineFrame {
    pub now: SystemTime,
    pub ball: Option<SimulatedBall>,
    pub robots: Players<Option<RobotSnapshot>>,
    pub robot_frames: BTreeMap<PlayerNumber, RobotFrame>,
    pub invariant_violations: Vec<InvariantViolation>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RobotFrame {
    pub world_state: WorldState,
    pub motion_command: MotionCommand,
    pub trace: NodeTrace,
    pub static_layout: NodeTrace,
    pub path_obstacles: Vec<PathObstacle>,
    pub time_since_last_switch: Duration,
    pub direction_difference: f32,
    pub walk_position: Option<Point2<Ground>>,
    pub voronoi_map: Option<VoronoiGrid>,
    pub voronoi_inputs: Vec<Pose2<Field>>,
    pub outgoing_messages: Vec<OutgoingMessage>,
}

impl RobotFrame {
    pub(crate) fn from_outputs(
        world_state: WorldState,
        tick_output: SimulatorBehaviorTickOutput,
        outgoing_messages: Vec<OutgoingMessage>,
    ) -> Self {
        Self {
            world_state,
            motion_command: tick_output.motion_command,
            trace: tick_output.trace,
            static_layout: tick_output.static_layout,
            path_obstacles: tick_output.path_obstacles,
            time_since_last_switch: tick_output.time_since_last_switch,
            direction_difference: tick_output.direction_difference,
            walk_position: tick_output.walk_position,
            voronoi_map: tick_output.voronoi_map,
            voronoi_inputs: tick_output.voronoi_inputs,
            outgoing_messages,
        }
    }
}

pub(crate) fn record_timeline_frame(
    clock: Res<SimulatorClock>,
    ball: Res<SimulatorBall>,
    robot_frames: Res<SimulatorRobotFrames>,
    current_violations: Res<SimulatorCurrentInvariantViolations>,
    mut timeline: ResMut<SimulatorTimeline>,
    robots: Query<(
        &SimulatorRobot,
        &SimulatorGroundToWorld,
        &SimulatorPrimaryState,
        &SimulatorFallDownState,
    )>,
) {
    timeline.frames.push(TimelineFrame {
        now: clock.now,
        ball: ball.state,
        robots: robot_snapshots_from_query(&robots),
        robot_frames: robot_frames.0.clone(),
        invariant_violations: current_violations.0.clone(),
    });
}

pub(crate) fn robot_snapshots_from_query(
    robots: &Query<(
        &SimulatorRobot,
        &SimulatorGroundToWorld,
        &SimulatorPrimaryState,
        &SimulatorFallDownState,
    )>,
) -> Players<Option<RobotSnapshot>> {
    let mut snapshots = Players::default();
    for (robot, ground_to_world, primary_state, fall_down_state) in robots.iter() {
        snapshots[robot.player_number] = Some(RobotSnapshot {
            player_number: robot.player_number,
            ground_to_world: ground_to_world.ground_to_world,
            primary_state: primary_state.primary_state,
            fall_down_state: fall_down_state.fall_down_state,
        });
    }
    snapshots
}

pub(crate) fn simulated_robot_snapshots(
    robots: &Players<Option<SimulatedRobot>>,
) -> Players<Option<RobotSnapshot>> {
    robots.as_ref().map(|robot| {
        robot.as_ref().map(|robot| RobotSnapshot {
            player_number: robot.player_number,
            ground_to_world: robot.ground_to_world,
            primary_state: robot.primary_state,
            fall_down_state: robot.fall_down_state,
        })
    })
}
