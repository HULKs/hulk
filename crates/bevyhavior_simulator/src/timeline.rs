use std::{
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
    time::Duration,
    time::SystemTime,
};

use bevy::prelude::*;
use coordinate_systems::{Field, Ground};
use eframe::egui::Color32;
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Point2, Pose2};
use serde::Serialize;
use types::{
    behavior_tree::NodeTrace, filtered_game_state::FilteredGameState, messages::OutgoingMessage,
    motion_command::MotionCommand, path_obstacles::PathObstacle, players::Players,
    world_state::WorldState,
};
use voronoi::VoronoiGrid;

use crate::behavior_tree_simulator::{
    InvariantViolation, RobotSnapshot, SimulatedBall, SimulatorBall, SimulatorBehaviorTickOutput,
    SimulatorClock, SimulatorCurrentInvariantViolations, SimulatorFallDownState,
    SimulatorGameState, SimulatorGroundToWorld, SimulatorHeadYaw, SimulatorPrimaryState,
    SimulatorRobot,
};
use crate::game_controller::filtered_game_state_from;

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorTimeline {
    pub frames: Vec<TimelineFrame>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorTimelineMarkers {
    pub markers: Vec<SimulatorTimelineMarker>,
}

impl SimulatorTimelineMarkers {
    pub fn add(&mut self, frame_time: SystemTime, color: Color32, label: impl Into<String>) {
        self.markers.push(SimulatorTimelineMarker {
            frame_time,
            color,
            label: label.into(),
        });
    }
}

#[derive(Clone, Debug)]
pub struct SimulatorTimelineMarker {
    pub frame_time: SystemTime,
    pub color: Color32,
    pub label: String,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorScenarioResult {
    pub failed: bool,
    pub failures: Vec<SimulatorFailure>,
}

#[derive(Clone, Debug, Serialize)]
pub enum SimulatorFailure {
    InvariantViolation(InvariantViolation),
}

impl Display for SimulatorFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvariantViolation(violation) => write!(formatter, "{}", violation),
        }
    }
}

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorRobotFrames(pub BTreeMap<PlayerNumber, RobotFrame>);

#[derive(Clone, Debug, Serialize)]
pub struct TimelineFrame {
    pub now: SystemTime,
    pub game_state: FilteredGameState,
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
    game_state: Res<SimulatorGameState>,
    robot_frames: Res<SimulatorRobotFrames>,
    current_violations: Res<SimulatorCurrentInvariantViolations>,
    mut timeline: ResMut<SimulatorTimeline>,
    robots: Query<(
        &SimulatorRobot,
        &SimulatorGroundToWorld,
        &SimulatorHeadYaw,
        &SimulatorPrimaryState,
        &SimulatorFallDownState,
    )>,
) {
    timeline.frames.push(TimelineFrame {
        now: clock.now,
        game_state: game_state
            .filtered_game_controller_state
            .as_ref()
            .map(|state| state.game_state)
            .unwrap_or_else(|| filtered_game_state_from(&game_state.game_controller_state)),
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
        &SimulatorHeadYaw,
        &SimulatorPrimaryState,
        &SimulatorFallDownState,
    )>,
) -> Players<Option<RobotSnapshot>> {
    let mut snapshots = Players::default();
    for (robot, ground_to_world, head_yaw, primary_state, fall_down_state) in robots.iter() {
        snapshots[robot.player_number] = Some(RobotSnapshot {
            player_number: robot.player_number,
            ground_to_world: ground_to_world.ground_to_world,
            head_yaw: head_yaw.yaw,
            primary_state: primary_state.primary_state,
            fall_down_state: fall_down_state.fall_down_state,
        });
    }
    snapshots
}
