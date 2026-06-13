use std::{collections::BTreeMap, env, net::SocketAddr, time::Duration, time::SystemTime};

use bevy::{
    app::{App, AppExit, Plugin, Update},
    ecs::message::Messages,
    prelude::*,
};
use booster::FallDownState;
use color_eyre::{Result, eyre::bail};
use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Orientation2, Point2, Pose2, Vector2};
use serde::{Deserialize, Serialize};
use types::{
    behavior_tree::NodeTrace,
    field_dimensions::{FieldDimensions, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    messages::OutgoingMessage,
    motion_command::{KickPower, MotionCommand, OrientationMode},
    parameters::{BehaviorParameters, HslNetworkParameters},
    path::PathSegment,
    path_obstacles::PathObstacle,
    players::Players,
    primary_state::PrimaryState,
    rule_obstacles::RuleObstacle,
    world_state::{BallState, PlayerState, RobotState, WorldState},
};
use voronoi::VoronoiGrid;
use world_state::behavior::{
    node::{Behavior, BehaviorTickInput, BehaviorTickOutput, CreationContext},
    send_message::CommunicationInput,
};

use crate::timeline_viewer::{TimelineViewerData, show_timeline_viewer};

pub const DEFAULT_TICK_DURATION: Duration = Duration::from_millis(10);
const PLAYER_NUMBERS: [PlayerNumber; 5] = [
    PlayerNumber::One,
    PlayerNumber::Two,
    PlayerNumber::Three,
    PlayerNumber::Four,
    PlayerNumber::Five,
];

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BehaviorTreeSimulatorSet {
    AdvanceTime,
    BeforeBallPhysics,
    BallPhysics,
    AfterBallPhysics,
    BuildTeamContext,
    BeforeWorldState,
    BuildWorldStates,
    AfterWorldState,
    BeforeBehavior,
    TickBehaviorTrees,
    AfterBehavior,
    BeforeCommunication,
    PlanCommunication,
    AfterCommunication,
    BeforeKinematics,
    ApplyKinematics,
    AfterKinematics,
    BeforeInvariantChecks,
    RunInvariantChecks,
    AfterInvariantChecks,
    RecordTimeline,
    Scenario,
}

#[derive(Resource, Clone, Debug)]
pub struct SimulationConfig {
    pub walk_translation_speed: f32,
    pub walk_rotation_speed: f32,
    pub walk_with_velocity_scale: f32,
    pub kick_ball_speed_rumpelstilzchen: f32,
    pub kick_ball_speed_schlong: f32,
    pub kick_cooldown: Duration,
    pub ball_friction_per_second: f32,
    pub ball_visibility_range: f32,
    pub ball_visibility_angle: f32,
    pub robot_radius: f32,
    pub kick_radius: f32,
    pub free_kick_obstacle_radius: f32,
    pub remaining_amount_of_messages: Option<u16>,
    pub game_controller_address: Option<SocketAddr>,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            walk_translation_speed: 0.25,
            walk_rotation_speed: 1.0,
            walk_with_velocity_scale: 1.0,
            kick_ball_speed_rumpelstilzchen: 2.0,
            kick_ball_speed_schlong: 4.0,
            kick_cooldown: Duration::from_millis(750),
            ball_friction_per_second: 0.4,
            ball_visibility_range: 4.0,
            ball_visibility_angle: std::f32::consts::FRAC_PI_2,
            robot_radius: 0.25,
            kick_radius: 0.25,
            free_kick_obstacle_radius: 0.75,
            remaining_amount_of_messages: Some(u16::MAX),
            game_controller_address: None,
        }
    }
}

pub struct BehaviorTreeSimulatorPlugin {
    pub config: SimulationConfig,
    pub field_dimensions: FieldDimensions,
    pub hsl_network_parameters: HslNetworkParameters,
    pub tick_duration: Duration,
    pub enable_default_ball_physics: bool,
    pub enable_default_kinematics: bool,
    pub enable_default_communication_routing: bool,
    pub enable_default_invariant_checks: bool,
}

impl Default for BehaviorTreeSimulatorPlugin {
    fn default() -> Self {
        Self {
            config: SimulationConfig::default(),
            field_dimensions: FieldDimensions::SPL_2025,
            hsl_network_parameters: HslNetworkParameters::default(),
            tick_duration: DEFAULT_TICK_DURATION,
            enable_default_ball_physics: true,
            enable_default_kinematics: true,
            enable_default_communication_routing: true,
            enable_default_invariant_checks: true,
        }
    }
}

impl Plugin for BehaviorTreeSimulatorPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<AppExit>()
            .insert_resource(SimulatorClock {
                now: SystemTime::UNIX_EPOCH,
                tick_duration: self.tick_duration,
            })
            .insert_resource(SimulatorFieldDimensions(self.field_dimensions))
            .insert_resource(SimulatorBall::default())
            .insert_resource(SimulatorGameState::default())
            .insert_resource(SimulatorRuleObstacles::default())
            .insert_resource(SimulatorHslNetworkParameters(
                self.hsl_network_parameters.clone(),
            ))
            .insert_resource(self.config.clone())
            .insert_resource(SimulatorTimeline::default())
            .insert_resource(SimulatorScenarioResult::default())
            .insert_resource(SimulatorIncomingMessages::default())
            .insert_resource(SimulatorOutgoingMessages::default())
            .insert_resource(SimulatorWorldStates::default())
            .insert_resource(SimulatorRobotFrames::default())
            .insert_resource(SimulatorCurrentInvariantViolations::default());

        if self.enable_default_invariant_checks {
            app.insert_resource(SimulatorInvariantChecks(default_invariant_checks()));
        } else {
            app.insert_resource(SimulatorInvariantChecks::default());
        }

        app.configure_sets(
            Update,
            (
                BehaviorTreeSimulatorSet::AdvanceTime,
                BehaviorTreeSimulatorSet::BeforeBallPhysics,
                BehaviorTreeSimulatorSet::BallPhysics,
                BehaviorTreeSimulatorSet::AfterBallPhysics,
                BehaviorTreeSimulatorSet::BuildTeamContext,
                BehaviorTreeSimulatorSet::BeforeWorldState,
                BehaviorTreeSimulatorSet::BuildWorldStates,
                BehaviorTreeSimulatorSet::AfterWorldState,
                BehaviorTreeSimulatorSet::BeforeBehavior,
                BehaviorTreeSimulatorSet::TickBehaviorTrees,
                BehaviorTreeSimulatorSet::AfterBehavior,
                BehaviorTreeSimulatorSet::BeforeCommunication,
                BehaviorTreeSimulatorSet::PlanCommunication,
                BehaviorTreeSimulatorSet::AfterCommunication,
            )
                .chain(),
        )
        .configure_sets(
            Update,
            (
                BehaviorTreeSimulatorSet::BeforeKinematics,
                BehaviorTreeSimulatorSet::ApplyKinematics,
                BehaviorTreeSimulatorSet::AfterKinematics,
                BehaviorTreeSimulatorSet::BeforeInvariantChecks,
                BehaviorTreeSimulatorSet::RunInvariantChecks,
                BehaviorTreeSimulatorSet::AfterInvariantChecks,
                BehaviorTreeSimulatorSet::RecordTimeline,
                BehaviorTreeSimulatorSet::Scenario,
            )
                .chain(),
        )
        .configure_sets(
            Update,
            BehaviorTreeSimulatorSet::BeforeKinematics
                .after(BehaviorTreeSimulatorSet::AfterCommunication),
        )
        .add_systems(
            Update,
            advance_time.in_set(BehaviorTreeSimulatorSet::AdvanceTime),
        )
        .add_systems(
            Update,
            build_world_states.in_set(BehaviorTreeSimulatorSet::BuildWorldStates),
        )
        .add_systems(
            Update,
            tick_behavior_trees.in_set(BehaviorTreeSimulatorSet::TickBehaviorTrees),
        )
        .add_systems(
            Update,
            plan_communication.in_set(BehaviorTreeSimulatorSet::PlanCommunication),
        )
        .add_systems(
            Update,
            run_invariant_checks.in_set(BehaviorTreeSimulatorSet::RunInvariantChecks),
        )
        .add_systems(
            Update,
            record_timeline_frame.in_set(BehaviorTreeSimulatorSet::RecordTimeline),
        );

        if self.enable_default_ball_physics {
            app.add_systems(
                Update,
                update_ball_kinematics.in_set(BehaviorTreeSimulatorSet::BallPhysics),
            );
        }

        if self.enable_default_communication_routing {
            app.add_systems(
                Update,
                route_outgoing_communication.in_set(BehaviorTreeSimulatorSet::AfterCommunication),
            );
        }

        if self.enable_default_kinematics {
            app.add_systems(
                Update,
                apply_motion_kinematics.in_set(BehaviorTreeSimulatorSet::ApplyKinematics),
            );
        }
    }
}

pub trait AppExt {
    fn run_to_completion(&mut self) -> Result<()>;
    fn run_to_completion_with_viewer(&mut self) -> Result<()>;
}

impl AppExt for App {
    fn run_to_completion(&mut self) -> Result<()> {
        let exit = run_until_exit(self);
        check_scenario_result(self, exit)
    }

    fn run_to_completion_with_viewer(&mut self) -> Result<()> {
        let exit = run_until_exit(self);

        let viewer_data = TimelineViewerData {
            field_dimensions: self.world().resource::<SimulatorFieldDimensions>().0,
            frames: self.world().resource::<SimulatorTimeline>().frames.clone(),
            failures: self
                .world()
                .resource::<SimulatorScenarioResult>()
                .failures
                .clone(),
        };

        if env::var_os("BEVYHAVIOR_SIMULATOR_NO_VIEWER").is_none() {
            show_timeline_viewer(viewer_data)?;
        }

        check_scenario_result(self, exit)
    }
}

fn run_until_exit(app: &mut App) -> AppExit {
    let mut event_cursor = app
        .world_mut()
        .resource_mut::<Messages<AppExit>>()
        .get_cursor();

    loop {
        app.update();

        let events = app.world().resource::<Messages<AppExit>>();
        if let Some(exit_message) = event_cursor.read(events).last() {
            break exit_message.clone();
        }
    }
}

fn check_scenario_result(app: &App, exit: AppExit) -> Result<()> {
    if let AppExit::Error(code) = exit {
        bail!("scenario exited with error code {code}");
    }

    let scenario_result = app.world().resource::<SimulatorScenarioResult>();
    if scenario_result.failed {
        bail!("{} simulator failure(s)", scenario_result.failures.len());
    }

    Ok(())
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct SimulatorClock {
    pub now: SystemTime,
    pub tick_duration: Duration,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct SimulatorFieldDimensions(pub FieldDimensions);

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct SimulatorBall {
    pub state: Option<SimulatedBall>,
}

#[derive(Resource, Clone, Debug)]
pub struct SimulatorGameState {
    pub filtered_game_controller_state: Option<FilteredGameControllerState>,
}

impl Default for SimulatorGameState {
    fn default() -> Self {
        Self {
            filtered_game_controller_state: Some(FilteredGameControllerState::default()),
        }
    }
}

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorRuleObstacles {
    pub obstacles: Vec<RuleObstacle>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorTimeline {
    pub frames: Vec<TimelineFrame>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorScenarioResult {
    pub failed: bool,
    pub failures: Vec<SimulatorFailure>,
}

#[derive(Clone, Debug)]
pub enum SimulatorFailure {
    InvariantViolation(InvariantViolation),
    ScenarioAssertion(String),
}

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorIncomingMessages {
    pub messages: Vec<SimulatorMessage>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorOutgoingMessages {
    pub messages: Vec<SimulatorMessage>,
}

#[derive(Clone, Debug)]
pub struct SimulatorMessage {
    pub sender: PlayerNumber,
    pub message: OutgoingMessage,
}

#[derive(Resource, Clone, Debug)]
pub struct SimulatorHslNetworkParameters(pub HslNetworkParameters);

#[derive(Resource, Default)]
pub struct SimulatorInvariantChecks(pub Vec<Box<dyn InvariantCheck>>);

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorWorldStates(pub BTreeMap<PlayerNumber, WorldState>);

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorRobotFrames(pub BTreeMap<PlayerNumber, RobotFrame>);

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorCurrentInvariantViolations(pub Vec<InvariantViolation>);

#[derive(Component, Clone, Copy, Debug)]
pub struct SimulatorRobot {
    pub player_number: PlayerNumber,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct SimulatorGroundToField {
    pub ground_to_field: Isometry2<Ground, Field>,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct SimulatorPrimaryState {
    pub primary_state: PrimaryState,
}

#[derive(Component)]
pub struct SimulatorRobotBehavior {
    pub behavior: Behavior,
}

#[derive(Component, Clone, Debug)]
pub struct SimulatorRobotParameters {
    pub behavior: BehaviorParameters,
}

#[derive(Component, Clone, Debug)]
pub struct SimulatorLastMotionCommand {
    pub motion_command: MotionCommand,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SimulatorFallDownState {
    pub fall_down_state: Option<FallDownState>,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SimulatorSuggestedSearchPosition {
    pub position: Option<Point2<Field>>,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct SimulatorLastKickTime {
    pub last_kick_time: SystemTime,
}

#[derive(Bundle)]
pub struct SimulatorRobotBundle {
    pub robot: SimulatorRobot,
    pub ground_to_field: SimulatorGroundToField,
    pub primary_state: SimulatorPrimaryState,
    pub behavior: SimulatorRobotBehavior,
    pub parameters: SimulatorRobotParameters,
    pub last_motion_command: SimulatorLastMotionCommand,
    pub fall_down_state: SimulatorFallDownState,
    pub suggested_search_position: SimulatorSuggestedSearchPosition,
    pub last_kick_time: SimulatorLastKickTime,
}

impl SimulatorRobotBundle {
    pub fn new(
        player_number: PlayerNumber,
        ground_to_field: Isometry2<Ground, Field>,
        parameters: BehaviorParameters,
    ) -> Result<Self> {
        Ok(Self {
            robot: SimulatorRobot { player_number },
            ground_to_field: SimulatorGroundToField { ground_to_field },
            primary_state: SimulatorPrimaryState {
                primary_state: PrimaryState::Safe,
            },
            behavior: SimulatorRobotBehavior {
                behavior: Behavior::new(CreationContext {})?,
            },
            parameters: SimulatorRobotParameters {
                behavior: parameters,
            },
            last_motion_command: SimulatorLastMotionCommand {
                motion_command: MotionCommand::default(),
            },
            fall_down_state: SimulatorFallDownState::default(),
            suggested_search_position: SimulatorSuggestedSearchPosition::default(),
            last_kick_time: SimulatorLastKickTime {
                last_kick_time: SystemTime::UNIX_EPOCH,
            },
        })
    }

    pub fn with_primary_state(mut self, primary_state: PrimaryState) -> Self {
        self.primary_state.primary_state = primary_state;
        self
    }
}

fn advance_time(mut clock: ResMut<SimulatorClock>) {
    let tick_duration = clock.tick_duration;
    clock.now += tick_duration;
}

fn update_ball_kinematics(
    clock: Res<SimulatorClock>,
    config: Res<SimulationConfig>,
    mut ball: ResMut<SimulatorBall>,
) {
    let Some(ball) = &mut ball.state else {
        return;
    };
    let dt = clock.tick_duration.as_secs_f32();
    ball.position += ball.velocity * dt;
    ball.velocity *= (1.0 - config.ball_friction_per_second * dt).clamp(0.0, 1.0);
}

fn build_world_states(
    clock: Res<SimulatorClock>,
    field_dimensions: Res<SimulatorFieldDimensions>,
    ball: Res<SimulatorBall>,
    game_state: Res<SimulatorGameState>,
    rule_obstacles: Res<SimulatorRuleObstacles>,
    config: Res<SimulationConfig>,
    robots: Query<(
        &SimulatorRobot,
        &SimulatorGroundToField,
        &SimulatorPrimaryState,
        &SimulatorFallDownState,
        &SimulatorSuggestedSearchPosition,
    )>,
    mut world_states: ResMut<SimulatorWorldStates>,
) {
    let player_states = player_states_from_query(&robots);
    world_states.0.clear();

    for (robot, ground_to_field, primary_state, fall_down_state, suggested_search_position) in
        &robots
    {
        let perceived_ball = perceived_ball_from_pose(
            ball.state,
            ground_to_field.ground_to_field,
            clock.now,
            &config,
        );

        world_states.0.insert(
            robot.player_number,
            WorldState {
                ball: perceived_ball,
                filtered_game_controller_state: game_state.filtered_game_controller_state.clone(),
                hypothetical_ball_positions: Vec::new(),
                now: clock.now.into(),
                obstacles: Vec::new(),
                player_states: player_states.clone(),
                position_of_interest: Point2::origin(),
                robot: RobotState {
                    ground_to_field: Some(ground_to_field.ground_to_field),
                    player_number: robot.player_number,
                    primary_state: primary_state.primary_state,
                },
                rule_ball: ball
                    .state
                    .map(|ball| ball.to_ball_state(ground_to_field.ground_to_field, clock.now)),
                rule_obstacles: rule_obstacles.obstacles.clone(),
                fall_down_state: fall_down_state.fall_down_state,
                suggested_search_position: suggested_search_position.position,
            },
        );
    }

    let _ = field_dimensions;
}

fn tick_behavior_trees(
    clock: Res<SimulatorClock>,
    field_dimensions: Res<SimulatorFieldDimensions>,
    config: Res<SimulationConfig>,
    world_states: Res<SimulatorWorldStates>,
    mut robot_frames: ResMut<SimulatorRobotFrames>,
    mut robots: Query<(
        &SimulatorRobot,
        &SimulatorRobotParameters,
        &mut SimulatorRobotBehavior,
        &mut SimulatorLastMotionCommand,
    )>,
) {
    robot_frames.0.clear();

    for (robot, parameters, mut behavior, mut last_motion_command) in &mut robots {
        let Some(world_state) = world_states.0.get(&robot.player_number).cloned() else {
            continue;
        };

        let tick_output = behavior
            .behavior
            .tick_behavior_tree(BehaviorTickInput {
                world_state: world_state.clone(),
                field_dimensions: field_dimensions.0,
                parameters: parameters.behavior.clone(),
                free_kick_obstacle_radius: config.free_kick_obstacle_radius,
                last_motion_command: last_motion_command.motion_command.clone(),
            })
            .expect("behavior tree tick should not fail in simulator");

        last_motion_command.motion_command = tick_output.motion_command.clone();
        robot_frames.0.insert(
            robot.player_number,
            RobotFrame::from_outputs(world_state, tick_output, Vec::new()),
        );
    }

    let _ = clock;
}

fn plan_communication(
    config: Res<SimulationConfig>,
    hsl_network_parameters: Res<SimulatorHslNetworkParameters>,
    world_states: Res<SimulatorWorldStates>,
    mut robot_frames: ResMut<SimulatorRobotFrames>,
    mut outgoing_messages: ResMut<SimulatorOutgoingMessages>,
    mut robots: Query<(&SimulatorRobot, &mut SimulatorRobotBehavior)>,
) {
    outgoing_messages.messages.clear();

    for (robot, mut behavior) in &mut robots {
        let Some(world_state) = world_states.0.get(&robot.player_number) else {
            continue;
        };

        let communication_output = behavior.behavior.plan_communication(CommunicationInput {
            world_state,
            game_controller_address: config.game_controller_address,
            hsl_network_parameters: &hsl_network_parameters.0,
            remaining_amount_of_messages: config.remaining_amount_of_messages,
        });

        if let Some(frame) = robot_frames.0.get_mut(&robot.player_number) {
            frame.outgoing_messages = communication_output.outgoing_messages.clone();
        }

        outgoing_messages
            .messages
            .extend(
                communication_output
                    .outgoing_messages
                    .into_iter()
                    .map(|message| SimulatorMessage {
                        sender: robot.player_number,
                        message,
                    }),
            );
    }
}

fn route_outgoing_communication(
    outgoing_messages: Res<SimulatorOutgoingMessages>,
    mut incoming_messages: ResMut<SimulatorIncomingMessages>,
) {
    incoming_messages.messages.clear();
    incoming_messages
        .messages
        .extend(outgoing_messages.messages.iter().cloned());
}

fn run_invariant_checks(
    clock: Res<SimulatorClock>,
    ball: Res<SimulatorBall>,
    field_dimensions: Res<SimulatorFieldDimensions>,
    rule_obstacles: Res<SimulatorRuleObstacles>,
    config: Res<SimulationConfig>,
    robot_frames: Res<SimulatorRobotFrames>,
    mut invariant_checks: ResMut<SimulatorInvariantChecks>,
    mut current_violations: ResMut<SimulatorCurrentInvariantViolations>,
    mut scenario_result: ResMut<SimulatorScenarioResult>,
    robots: Query<(
        &SimulatorRobot,
        &SimulatorGroundToField,
        &SimulatorPrimaryState,
        &SimulatorFallDownState,
    )>,
) {
    current_violations.0.clear();

    let snapshot = SimulationSnapshot {
        now: clock.now,
        ball: ball.state,
        robots: robot_snapshots_from_query(&robots),
        robot_frames: robot_frames.0.clone(),
        field_dimensions: field_dimensions.0,
        rule_obstacles: rule_obstacles.obstacles.clone(),
        config: config.clone(),
    };

    for check in &mut invariant_checks.0 {
        current_violations.0.extend(check.check(&snapshot));
    }

    if !current_violations.0.is_empty() {
        scenario_result.failed = true;
        scenario_result.failures.extend(
            current_violations
                .0
                .iter()
                .cloned()
                .map(SimulatorFailure::InvariantViolation),
        );
    }
}

fn apply_motion_kinematics(
    clock: Res<SimulatorClock>,
    config: Res<SimulationConfig>,
    robot_frames: Res<SimulatorRobotFrames>,
    mut ball: ResMut<SimulatorBall>,
    mut robots: Query<(
        &SimulatorRobot,
        &mut SimulatorGroundToField,
        &mut SimulatorFallDownState,
        &mut SimulatorLastKickTime,
    )>,
) {
    for (robot, mut ground_to_field, mut fall_down_state, mut last_kick_time) in &mut robots {
        let Some(frame) = robot_frames.0.get(&robot.player_number) else {
            continue;
        };

        match &frame.motion_command {
            MotionCommand::Walk {
                path,
                orientation_mode,
                target_orientation,
                speed,
                ..
            } => {
                let target = first_path_target(path).unwrap_or_else(Point2::origin);
                ground_to_field.ground_to_field = apply_walk_to_pose(
                    ground_to_field.ground_to_field,
                    target,
                    *target_orientation,
                    *orientation_mode,
                    *speed,
                    clock.tick_duration,
                    &config,
                );
            }
            MotionCommand::WalkWithVelocity {
                velocity,
                angular_velocity,
                ..
            } => {
                ground_to_field.ground_to_field = apply_walk_with_velocity_to_pose(
                    ground_to_field.ground_to_field,
                    *velocity,
                    *angular_velocity,
                    clock.tick_duration,
                    &config,
                );
            }
            MotionCommand::VisualKick {
                ball_position,
                kick_direction,
                kick_power,
                ..
            } => apply_kick_to_ball(
                clock.now,
                &mut ball.state,
                &config,
                ground_to_field.ground_to_field,
                &mut last_kick_time.last_kick_time,
                *ball_position,
                *kick_direction,
                *kick_power,
            ),
            MotionCommand::StandUp => fall_down_state.fall_down_state = None,
            MotionCommand::Prepare | MotionCommand::Stand { .. } => {}
        }
    }
}

fn record_timeline_frame(
    clock: Res<SimulatorClock>,
    ball: Res<SimulatorBall>,
    robot_frames: Res<SimulatorRobotFrames>,
    current_violations: Res<SimulatorCurrentInvariantViolations>,
    mut timeline: ResMut<SimulatorTimeline>,
    robots: Query<(
        &SimulatorRobot,
        &SimulatorGroundToField,
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

fn player_states_from_query(
    robots: &Query<(
        &SimulatorRobot,
        &SimulatorGroundToField,
        &SimulatorPrimaryState,
        &SimulatorFallDownState,
        &SimulatorSuggestedSearchPosition,
    )>,
) -> Players<Option<PlayerState>> {
    let mut player_states = Players::default();
    for (robot, ground_to_field, _, _, _) in robots.iter() {
        player_states[robot.player_number] = Some(PlayerState {
            pose: ground_to_field.ground_to_field.as_pose(),
            ball_position: None,
        });
    }
    player_states
}

fn robot_snapshots_from_query(
    robots: &Query<(
        &SimulatorRobot,
        &SimulatorGroundToField,
        &SimulatorPrimaryState,
        &SimulatorFallDownState,
    )>,
) -> Players<Option<RobotSnapshot>> {
    let mut snapshots = Players::default();
    for (robot, ground_to_field, primary_state, fall_down_state) in robots.iter() {
        snapshots[robot.player_number] = Some(RobotSnapshot {
            player_number: robot.player_number,
            ground_to_field: ground_to_field.ground_to_field,
            primary_state: primary_state.primary_state,
            fall_down_state: fall_down_state.fall_down_state,
        });
    }
    snapshots
}

pub struct Simulation {
    pub now: SystemTime,
    pub tick_duration: Duration,
    pub robots: Players<Option<SimulatedRobot>>,
    pub ball: Option<SimulatedBall>,
    pub filtered_game_controller_state: Option<FilteredGameControllerState>,
    pub field_dimensions: FieldDimensions,
    pub rule_obstacles: Vec<RuleObstacle>,
    pub hsl_network_parameters: HslNetworkParameters,
    pub config: SimulationConfig,
    pub timeline: Vec<TimelineFrame>,
    pub invariant_checks: Vec<Box<dyn InvariantCheck>>,
    pub failed: bool,
}

impl Simulation {
    pub fn new(field_dimensions: FieldDimensions) -> Self {
        Self {
            now: SystemTime::UNIX_EPOCH,
            tick_duration: DEFAULT_TICK_DURATION,
            robots: Players::default(),
            ball: None,
            filtered_game_controller_state: Some(FilteredGameControllerState::default()),
            field_dimensions,
            rule_obstacles: Vec::new(),
            hsl_network_parameters: HslNetworkParameters::default(),
            config: SimulationConfig::default(),
            timeline: Vec::new(),
            invariant_checks: default_invariant_checks(),
            failed: false,
        }
    }

    pub fn with_config(mut self, config: SimulationConfig) -> Self {
        self.config = config;
        self
    }

    pub fn spawn_robot(
        &mut self,
        player_number: PlayerNumber,
        ground_to_field: Isometry2<Ground, Field>,
        parameters: BehaviorParameters,
    ) -> Result<()> {
        self.robots[player_number] = Some(SimulatedRobot::new(
            player_number,
            ground_to_field,
            parameters,
        )?);
        Ok(())
    }

    pub fn set_primary_state(&mut self, primary_state: PrimaryState) {
        for player_number in PLAYER_NUMBERS {
            if let Some(robot) = &mut self.robots[player_number] {
                robot.primary_state = primary_state;
            }
        }
    }

    pub fn set_ball(&mut self, position: Point2<Field>, velocity: Vector2<Field>) {
        self.ball = Some(SimulatedBall {
            position,
            velocity,
            field_side: Side::Left,
        });
    }

    pub fn add_invariant_check(&mut self, check: impl InvariantCheck + 'static) {
        self.invariant_checks.push(Box::new(check));
    }

    pub fn run_for(&mut self, duration: Duration) -> Result<()> {
        let ticks = duration.as_secs_f32() / self.tick_duration.as_secs_f32();
        self.run_ticks(ticks.ceil() as usize)
    }

    pub fn run_ticks(&mut self, ticks: usize) -> Result<()> {
        for _ in 0..ticks {
            self.tick()?;
        }
        Ok(())
    }

    pub fn tick(&mut self) -> Result<&TimelineFrame> {
        self.now += self.tick_duration;
        self.update_ball();

        let world_states = self.build_world_states();
        let mut robot_frames = BTreeMap::new();

        for player_number in PLAYER_NUMBERS {
            let Some(robot) = self.robots[player_number].as_mut() else {
                continue;
            };
            let Some(world_state) = world_states.get(&player_number).cloned() else {
                continue;
            };

            let tick_output = robot.behavior.tick_behavior_tree(BehaviorTickInput {
                world_state: world_state.clone(),
                field_dimensions: self.field_dimensions,
                parameters: robot.parameters.clone(),
                free_kick_obstacle_radius: self.config.free_kick_obstacle_radius,
                last_motion_command: robot.last_motion_command.clone(),
            })?;

            let communication_output = robot.behavior.plan_communication(CommunicationInput {
                world_state: &world_state,
                game_controller_address: self.config.game_controller_address,
                hsl_network_parameters: &self.hsl_network_parameters,
                remaining_amount_of_messages: self.config.remaining_amount_of_messages,
            });

            robot.last_motion_command = tick_output.motion_command.clone();

            robot_frames.insert(
                player_number,
                RobotFrame::from_outputs(
                    world_state,
                    tick_output,
                    communication_output.outgoing_messages,
                ),
            );
        }

        let mut snapshot = SimulationSnapshot {
            now: self.now,
            ball: self.ball,
            robots: simulated_robot_snapshots(&self.robots),
            robot_frames: robot_frames.clone(),
            field_dimensions: self.field_dimensions,
            rule_obstacles: self.rule_obstacles.clone(),
            config: self.config.clone(),
        };

        let mut invariant_violations = Vec::new();
        for check in &mut self.invariant_checks {
            invariant_violations.extend(check.check(&snapshot));
        }
        if !invariant_violations.is_empty() {
            self.failed = true;
        }

        self.apply_motion_commands(&robot_frames);
        snapshot.ball = self.ball;
        snapshot.robots = simulated_robot_snapshots(&self.robots);

        self.timeline.push(TimelineFrame {
            now: self.now,
            ball: self.ball,
            robots: snapshot.robots,
            robot_frames,
            invariant_violations,
        });

        Ok(self
            .timeline
            .last()
            .expect("timeline frame was just pushed"))
    }

    fn update_ball(&mut self) {
        let Some(ball) = &mut self.ball else { return };
        let dt = self.tick_duration.as_secs_f32();
        ball.position += ball.velocity * dt;
        ball.velocity *= (1.0 - self.config.ball_friction_per_second * dt).clamp(0.0, 1.0);
    }

    fn build_world_states(&self) -> BTreeMap<PlayerNumber, WorldState> {
        let player_states = self.player_states();
        let mut world_states = BTreeMap::new();

        for (player_number, robot) in self.robots.iter() {
            let Some(robot) = robot else { continue };
            let perceived_ball = self.perceived_ball(robot);

            world_states.insert(
                player_number,
                WorldState {
                    ball: perceived_ball,
                    filtered_game_controller_state: self.filtered_game_controller_state.clone(),
                    hypothetical_ball_positions: Vec::new(),
                    now: self.now.into(),
                    obstacles: Vec::new(),
                    player_states: player_states.clone(),
                    position_of_interest: Point2::origin(),
                    robot: RobotState {
                        ground_to_field: Some(robot.ground_to_field),
                        player_number,
                        primary_state: robot.primary_state,
                    },
                    rule_ball: self
                        .ball
                        .map(|ball| ball.to_ball_state(robot.ground_to_field, self.now)),
                    rule_obstacles: self.rule_obstacles.clone(),
                    fall_down_state: robot.fall_down_state,
                    suggested_search_position: robot.suggested_search_position,
                },
            );
        }

        world_states
    }

    fn player_states(&self) -> Players<Option<PlayerState>> {
        self.robots.as_ref().map(|robot| {
            robot.as_ref().map(|robot| PlayerState {
                pose: robot.ground_to_field.as_pose(),
                ball_position: None,
            })
        })
    }

    fn perceived_ball(&self, robot: &SimulatedRobot) -> Option<BallState> {
        let ball = self.ball?;
        let ball_in_ground = robot.ground_to_field.inverse() * ball.position;
        let distance = ball_in_ground.coords().norm();
        if distance > self.config.ball_visibility_range {
            return None;
        }

        let angle = ball_in_ground.coords().angle(&Vector2::x_axis());
        if angle.abs() > self.config.ball_visibility_angle / 2.0 {
            return None;
        }

        Some(ball.to_ball_state(robot.ground_to_field, self.now))
    }

    fn apply_motion_commands(&mut self, robot_frames: &BTreeMap<PlayerNumber, RobotFrame>) {
        let robots = &mut self.robots;
        let ball = &mut self.ball;
        let now = self.now;
        let tick_duration = self.tick_duration;
        let config = &self.config;

        for (player_number, frame) in robot_frames {
            let Some(robot) = robots[*player_number].as_mut() else {
                continue;
            };

            match &frame.motion_command {
                MotionCommand::Walk {
                    path,
                    orientation_mode,
                    target_orientation,
                    speed,
                    ..
                } => {
                    let target = first_path_target(path).unwrap_or_else(Point2::origin);
                    apply_walk(
                        robot,
                        target,
                        *target_orientation,
                        *orientation_mode,
                        *speed,
                        tick_duration,
                        config,
                    );
                }
                MotionCommand::WalkWithVelocity {
                    velocity,
                    angular_velocity,
                    ..
                } => apply_walk_with_velocity(
                    robot,
                    *velocity,
                    *angular_velocity,
                    tick_duration,
                    config,
                ),
                MotionCommand::VisualKick {
                    ball_position,
                    kick_direction,
                    kick_power,
                    ..
                } => apply_kick(
                    now,
                    ball,
                    config,
                    robot,
                    *ball_position,
                    *kick_direction,
                    *kick_power,
                ),
                MotionCommand::StandUp => robot.fall_down_state = None,
                MotionCommand::Prepare | MotionCommand::Stand { .. } => {}
            }
        }
    }
}

pub struct SimulatedRobot {
    pub player_number: PlayerNumber,
    pub ground_to_field: Isometry2<Ground, Field>,
    pub primary_state: PrimaryState,
    pub behavior: Behavior,
    pub parameters: BehaviorParameters,
    pub last_motion_command: MotionCommand,
    pub fall_down_state: Option<FallDownState>,
    pub suggested_search_position: Option<Point2<Field>>,
    pub last_kick_time: SystemTime,
}

impl SimulatedRobot {
    pub fn new(
        player_number: PlayerNumber,
        ground_to_field: Isometry2<Ground, Field>,
        parameters: BehaviorParameters,
    ) -> Result<Self> {
        Ok(Self {
            player_number,
            ground_to_field,
            primary_state: PrimaryState::Safe,
            behavior: Behavior::new(CreationContext {})?,
            parameters,
            last_motion_command: MotionCommand::default(),
            fall_down_state: None,
            suggested_search_position: None,
            last_kick_time: SystemTime::UNIX_EPOCH,
        })
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SimulatedBall {
    pub position: Point2<Field>,
    pub velocity: Vector2<Field>,
    pub field_side: Side,
}

impl SimulatedBall {
    fn to_ball_state(
        self,
        ground_to_field: Isometry2<Ground, Field>,
        now: SystemTime,
    ) -> BallState {
        BallState {
            ball_in_ground: ground_to_field.inverse() * self.position,
            ball_in_field: self.position,
            ball_in_ground_velocity: ground_to_field.inverse() * self.velocity,
            last_seen_ball: now,
            field_side: self.field_side,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TimelineFrame {
    pub now: SystemTime,
    pub ball: Option<SimulatedBall>,
    pub robots: Players<Option<RobotSnapshot>>,
    pub robot_frames: BTreeMap<PlayerNumber, RobotFrame>,
    pub invariant_violations: Vec<InvariantViolation>,
}

#[derive(Clone, Debug)]
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
    fn from_outputs(
        world_state: WorldState,
        tick_output: BehaviorTickOutput,
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

#[derive(Clone, Copy, Debug)]
pub struct RobotSnapshot {
    pub player_number: PlayerNumber,
    pub ground_to_field: Isometry2<Ground, Field>,
    pub primary_state: PrimaryState,
    pub fall_down_state: Option<FallDownState>,
}

#[derive(Clone, Debug)]
pub struct SimulationSnapshot {
    pub now: SystemTime,
    pub ball: Option<SimulatedBall>,
    pub robots: Players<Option<RobotSnapshot>>,
    pub robot_frames: BTreeMap<PlayerNumber, RobotFrame>,
    pub field_dimensions: FieldDimensions,
    pub rule_obstacles: Vec<RuleObstacle>,
    pub config: SimulationConfig,
}

#[derive(Clone, Debug)]
pub struct InvariantViolation {
    pub check_name: &'static str,
    pub player_number: Option<PlayerNumber>,
    pub message: String,
    pub severity: InvariantSeverity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvariantSeverity {
    Warning,
    Error,
}

pub trait InvariantCheck: Send + Sync {
    fn check(&mut self, snapshot: &SimulationSnapshot) -> Vec<InvariantViolation>;
}

pub fn default_invariant_checks() -> Vec<Box<dyn InvariantCheck>> {
    vec![
        Box::new(RuleObstacleWalkCheck),
        Box::new(FieldBoundaryWalkCheck),
    ]
}

pub struct RuleObstacleWalkCheck;

impl InvariantCheck for RuleObstacleWalkCheck {
    fn check(&mut self, snapshot: &SimulationSnapshot) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();
        for (player_number, frame) in &snapshot.robot_frames {
            let Some(target) = motion_target_in_field(frame) else {
                continue;
            };

            for obstacle in &frame.world_state.rule_obstacles {
                if obstacle.contains(target) {
                    violations.push(InvariantViolation {
                        check_name: "rule_obstacle_walk",
                        player_number: Some(*player_number),
                        message: format!(
                            "robot {player_number:?} plans to walk into a known rule obstacle"
                        ),
                        severity: InvariantSeverity::Error,
                    });
                    break;
                }
            }
        }
        violations
    }
}

pub struct FieldBoundaryWalkCheck;

impl InvariantCheck for FieldBoundaryWalkCheck {
    fn check(&mut self, snapshot: &SimulationSnapshot) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();
        for (player_number, frame) in &snapshot.robot_frames {
            let Some(target) = motion_target_in_field(frame) else {
                continue;
            };

            if !snapshot.field_dimensions.is_inside_field(target) {
                violations.push(InvariantViolation {
                    check_name: "field_boundary_walk",
                    player_number: Some(*player_number),
                    message: format!(
                        "robot {player_number:?} plans to walk outside the known field"
                    ),
                    severity: InvariantSeverity::Error,
                });
            }
        }
        violations
    }
}

fn simulated_robot_snapshots(
    robots: &Players<Option<SimulatedRobot>>,
) -> Players<Option<RobotSnapshot>> {
    robots.as_ref().map(|robot| {
        robot.as_ref().map(|robot| RobotSnapshot {
            player_number: robot.player_number,
            ground_to_field: robot.ground_to_field,
            primary_state: robot.primary_state,
            fall_down_state: robot.fall_down_state,
        })
    })
}

fn perceived_ball_from_pose(
    ball: Option<SimulatedBall>,
    ground_to_field: Isometry2<Ground, Field>,
    now: SystemTime,
    config: &SimulationConfig,
) -> Option<BallState> {
    let ball = ball?;
    let ball_in_ground = ground_to_field.inverse() * ball.position;
    let distance = ball_in_ground.coords().norm();
    if distance > config.ball_visibility_range {
        return None;
    }

    let angle = ball_in_ground.coords().angle(&Vector2::x_axis());
    if angle.abs() > config.ball_visibility_angle / 2.0 {
        return None;
    }

    Some(ball.to_ball_state(ground_to_field, now))
}

fn motion_target_in_field(frame: &RobotFrame) -> Option<Point2<Field>> {
    let MotionCommand::Walk { path, .. } = &frame.motion_command else {
        return None;
    };
    let ground_to_field = frame.world_state.robot.ground_to_field?;
    first_path_target(path).map(|target| ground_to_field * target)
}

fn first_path_target(path: &types::path::Path) -> Option<Point2<Ground>> {
    let segment = path.segments.first()?;
    match segment {
        PathSegment::LineSegment(segment) => Some(segment.1),
        PathSegment::Arc(arc) => {
            Some(arc.circle.center + arc.end.as_unit_vector() * arc.circle.radius)
        }
    }
}

fn apply_walk(
    robot: &mut SimulatedRobot,
    target: Point2<Ground>,
    target_orientation: Orientation2<Ground>,
    orientation_mode: OrientationMode,
    speed: f32,
    tick_duration: Duration,
    config: &SimulationConfig,
) {
    let dt = tick_duration.as_secs_f32();
    let max_distance = config.walk_translation_speed * speed * dt;
    let target_vector = target.coords();
    let step_translation =
        if target_vector.norm() > max_distance && target_vector.norm() > f32::EPSILON {
            target_vector.normalize() * max_distance
        } else {
            target_vector
        };

    let desired_orientation = match orientation_mode {
        OrientationMode::LookTowards { direction, .. } => direction,
        OrientationMode::LookAt { target, .. } => Orientation2::from_vector(target.coords()),
        OrientationMode::AlignWithPath | OrientationMode::Unspecified => target_orientation,
    };
    let max_rotation = config.walk_rotation_speed * dt;
    let step_rotation = desired_orientation
        .angle()
        .clamp(-max_rotation, max_rotation);
    let delta = Isometry2::from_parts(step_translation, step_rotation);
    robot.ground_to_field = robot.ground_to_field * delta;
}

fn apply_walk_to_pose(
    ground_to_field: Isometry2<Ground, Field>,
    target: Point2<Ground>,
    target_orientation: Orientation2<Ground>,
    orientation_mode: OrientationMode,
    speed: f32,
    tick_duration: Duration,
    config: &SimulationConfig,
) -> Isometry2<Ground, Field> {
    let dt = tick_duration.as_secs_f32();
    let max_distance = config.walk_translation_speed * speed * dt;
    let target_vector = target.coords();
    let step_translation =
        if target_vector.norm() > max_distance && target_vector.norm() > f32::EPSILON {
            target_vector.normalize() * max_distance
        } else {
            target_vector
        };

    let desired_orientation = match orientation_mode {
        OrientationMode::LookTowards { direction, .. } => direction,
        OrientationMode::LookAt { target, .. } => Orientation2::from_vector(target.coords()),
        OrientationMode::AlignWithPath | OrientationMode::Unspecified => target_orientation,
    };
    let max_rotation = config.walk_rotation_speed * dt;
    let step_rotation = desired_orientation
        .angle()
        .clamp(-max_rotation, max_rotation);
    let delta = Isometry2::from_parts(step_translation, step_rotation);
    ground_to_field * delta
}

fn apply_walk_with_velocity(
    robot: &mut SimulatedRobot,
    velocity: Vector2<Ground>,
    angular_velocity: f32,
    tick_duration: Duration,
    config: &SimulationConfig,
) {
    let dt = tick_duration.as_secs_f32();
    let translation = velocity * config.walk_with_velocity_scale * dt;
    let rotation = angular_velocity * config.walk_with_velocity_scale * dt;
    let delta = Isometry2::from_parts(translation, rotation);
    robot.ground_to_field = robot.ground_to_field * delta;
}

fn apply_walk_with_velocity_to_pose(
    ground_to_field: Isometry2<Ground, Field>,
    velocity: Vector2<Ground>,
    angular_velocity: f32,
    tick_duration: Duration,
    config: &SimulationConfig,
) -> Isometry2<Ground, Field> {
    let dt = tick_duration.as_secs_f32();
    let translation = velocity * config.walk_with_velocity_scale * dt;
    let rotation = angular_velocity * config.walk_with_velocity_scale * dt;
    let delta = Isometry2::from_parts(translation, rotation);
    ground_to_field * delta
}

fn apply_kick(
    now: SystemTime,
    ball: &mut Option<SimulatedBall>,
    config: &SimulationConfig,
    robot: &mut SimulatedRobot,
    expected_ball_position: Point2<Ground>,
    kick_direction: Orientation2<Ground>,
    kick_power: KickPower,
) {
    let Some(ball) = ball else { return };
    if now.duration_since(robot.last_kick_time).unwrap_or_default() < config.kick_cooldown {
        return;
    }

    let expected_ball_in_field = robot.ground_to_field * expected_ball_position;
    if (ball.position - expected_ball_in_field).norm() > config.kick_radius {
        return;
    }

    let speed = match kick_power {
        KickPower::Rumpelstilzchen => config.kick_ball_speed_rumpelstilzchen,
        KickPower::Schlong => config.kick_ball_speed_schlong,
    };
    ball.velocity = robot.ground_to_field * (kick_direction.as_unit_vector() * speed);
    robot.last_kick_time = now;
}

fn apply_kick_to_ball(
    now: SystemTime,
    ball: &mut Option<SimulatedBall>,
    config: &SimulationConfig,
    ground_to_field: Isometry2<Ground, Field>,
    last_kick_time: &mut SystemTime,
    expected_ball_position: Point2<Ground>,
    kick_direction: Orientation2<Ground>,
    kick_power: KickPower,
) {
    let Some(ball) = ball else { return };
    if now.duration_since(*last_kick_time).unwrap_or_default() < config.kick_cooldown {
        return;
    }

    let expected_ball_in_field = ground_to_field * expected_ball_position;
    if (ball.position - expected_ball_in_field).norm() > config.kick_radius {
        return;
    }

    let speed = match kick_power {
        KickPower::Rumpelstilzchen => config.kick_ball_speed_rumpelstilzchen,
        KickPower::Schlong => config.kick_ball_speed_schlong,
    };
    ball.velocity = ground_to_field * (kick_direction.as_unit_vector() * speed);
    *last_kick_time = now;
}
