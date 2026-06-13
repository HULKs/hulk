use std::{collections::BTreeMap, time::Duration, time::SystemTime};

use color_eyre::Result;
use coordinate_systems::{Ground, World};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Point2, Vector2};
use types::{
    field_dimensions::{FieldDimensions, GlobalFieldSide, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::MotionCommand,
    parameters::{BehaviorParameters, HslNetworkParameters},
    players::Players,
    primary_state::PrimaryState,
    rule_obstacles::RuleObstacle,
    world_state::{BallState, PlayerState, RobotState, WorldState},
};

use crate::{
    behavior_runtime::SimulatorBehaviorTickInput,
    behavior_tree_simulator::{
        InvariantCheck, RobotFrame, SimulatedBall, SimulatedRobot, SimulationConfig,
        SimulationSnapshot, TimelineFrame,
    },
    config::DEFAULT_TICK_DURATION,
    coordinates::ground_to_field_from_world,
    game_controller::{default_game_controller_state, filtered_game_controller_state_from},
    invariant_checks::default_invariant_checks,
    kinematics::{apply_kick, apply_walk, apply_walk_with_velocity, first_path_target},
    timeline::simulated_robot_snapshots,
};

const PLAYER_NUMBERS: [PlayerNumber; 5] = [
    PlayerNumber::One,
    PlayerNumber::Two,
    PlayerNumber::Three,
    PlayerNumber::Four,
    PlayerNumber::Five,
];

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
        let game_controller_state = default_game_controller_state();
        Self {
            now: SystemTime::UNIX_EPOCH,
            tick_duration: DEFAULT_TICK_DURATION,
            robots: Players::default(),
            ball: None,
            filtered_game_controller_state: Some(filtered_game_controller_state_from(
                &game_controller_state,
            )),
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
        ground_to_world: Isometry2<Ground, World>,
        parameters: BehaviorParameters,
    ) -> Result<()> {
        self.robots[player_number] = Some(SimulatedRobot::new(
            player_number,
            ground_to_world,
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

    pub fn set_ball(&mut self, position: Point2<World>, velocity: Vector2<World>) {
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

            let tick_output = robot
                .behavior
                .tick_behavior_tree(SimulatorBehaviorTickInput {
                    world_state: world_state.clone(),
                    field_dimensions: self.field_dimensions,
                    parameters: robot.parameters.clone(),
                })?;

            let outgoing_messages = robot.behavior.plan_communication(
                world_state.clone(),
                self.hsl_network_parameters.clone(),
                self.config.game_controller_address,
            );

            robot_frames.insert(
                player_number,
                RobotFrame::from_outputs(world_state, tick_output, outgoing_messages),
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
        let global_field_side = self.global_field_side();
        let player_states = self.player_states(global_field_side);
        let mut world_states = BTreeMap::new();

        for (player_number, robot) in self.robots.iter() {
            let Some(robot) = robot else { continue };
            let ground_to_field =
                ground_to_field_from_world(robot.ground_to_world, global_field_side);
            let perceived_ball = self.perceived_ball(robot, global_field_side);

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
                        ground_to_field: Some(ground_to_field),
                        player_number,
                        primary_state: robot.primary_state,
                    },
                    rule_ball: self.ball.map(|ball| {
                        ball.to_ball_state(robot.ground_to_world, global_field_side, self.now)
                    }),
                    rule_obstacles: self.rule_obstacles.clone(),
                    fall_down_state: robot.fall_down_state,
                    suggested_search_position: robot.suggested_search_position,
                },
            );
        }

        world_states
    }

    fn player_states(&self, global_field_side: GlobalFieldSide) -> Players<Option<PlayerState>> {
        self.robots.as_ref().map(|robot| {
            robot.as_ref().map(|robot| PlayerState {
                pose: ground_to_field_from_world(robot.ground_to_world, global_field_side)
                    .as_pose(),
                ball_position: None,
            })
        })
    }

    fn perceived_ball(
        &self,
        robot: &SimulatedRobot,
        global_field_side: GlobalFieldSide,
    ) -> Option<BallState> {
        let ball = self.ball?;
        let ball_in_ground = robot.ground_to_world.inverse() * ball.position;
        let distance = ball_in_ground.coords().norm();
        if distance > self.config.ball_visibility_range {
            return None;
        }

        let angle = ball_in_ground.coords().angle(&Vector2::x_axis());
        if angle.abs() > self.config.ball_visibility_angle / 2.0 {
            return None;
        }

        Some(ball.to_ball_state(robot.ground_to_world, global_field_side, self.now))
    }

    fn global_field_side(&self) -> GlobalFieldSide {
        self.filtered_game_controller_state
            .as_ref()
            .map(|state| state.global_field_side)
            .unwrap_or(GlobalFieldSide::Home)
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
