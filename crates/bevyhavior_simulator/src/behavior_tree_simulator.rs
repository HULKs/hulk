use std::{collections::BTreeMap, net::SocketAddr, time::Duration, time::SystemTime};

use booster::FallDownState;
use color_eyre::Result;
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

pub const DEFAULT_TICK_DURATION: Duration = Duration::from_millis(100);
const PLAYER_NUMBERS: [PlayerNumber; 5] = [
    PlayerNumber::One,
    PlayerNumber::Two,
    PlayerNumber::Three,
    PlayerNumber::Four,
    PlayerNumber::Five,
];

#[derive(Clone, Debug)]
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
                    now: self.now,
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

pub trait InvariantCheck {
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
