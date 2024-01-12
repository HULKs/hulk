use std::{
    collections::{BTreeMap, HashMap},
    mem::take,
    time::{Duration, UNIX_EPOCH},
};

use color_eyre::Result;
use geometry::line_segment::LineSegment;
use nalgebra::{vector, Isometry2, Point2, UnitComplex, Vector2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use spl_network_messages::{GamePhase, GameState, HulkMessage, PlayerNumber, Team};
use types::{motion_command::KickVariant, planned_path::PathSegment, filtered_game_controller_state::FilteredGameControllerState};
use types::motion_command::{HeadMotion, OrientationMode};
use types::{
    ball_position::BallPosition,
    filtered_game_states::FilteredGameState,
    game_controller_state::GameControllerState,
    messages::{IncomingMessage, OutgoingMessage},
    motion_command::MotionCommand,
    players::Players,
    primary_state::PrimaryState,
    support_foot::Side,
};

use crate::{
    cycler::Database,
    robot::Robot,
    structs::{control::AdditionalOutputs, Parameters},
};

pub enum Event {
    Cycle,
    Goal,
}

#[derive(Default, Clone, Deserialize, Serialize, SerializeHierarchy)]
pub struct Ball {
    pub position: Point2<f32>,
    pub velocity: Vector2<f32>,
}

pub struct State {
    pub time_elapsed: Duration,
    pub cycle_count: usize,
    pub robots: HashMap<PlayerNumber, Robot>,
    pub ball: Option<Ball>,
    pub messages: Vec<(PlayerNumber, HulkMessage)>,
    pub finished: bool,
    pub game_controller_state: GameControllerState,
    pub filtered_game_state: FilteredGameState,
}

impl State {
    pub fn cycle(&mut self, time_step: Duration) -> Result<Vec<Event>> {
        let now = UNIX_EPOCH + self.time_elapsed;

        let mut events = vec![Event::Cycle];

        self.move_robots(time_step);
        self.cycle_robots(now)?;
        events.extend(self.move_ball(time_step));

        self.time_elapsed += time_step;
        self.cycle_count += 1;

        Ok(events)
    }

    fn move_robots(&mut self, time_step: Duration) {
        for robot in self.robots.values_mut() {
            let robot_to_field = robot
                .database
                .main_outputs
                .robot_to_field
                .as_mut()
                .expect("simulated robots should always have a known pose");

            robot.database.additional_outputs = AdditionalOutputs::default();
            let head_motion = match &robot.database.main_outputs.motion_command {
                MotionCommand::Walk {
                    head,
                    path,
                    orientation_mode,
                    ..
                } => {
                    let step = match path[0] {
                        PathSegment::LineSegment(LineSegment(_start, end)) => end.coords,
                        PathSegment::Arc(arc, orientation) => {
                            orientation.rotate_vector_90_degrees(arc.start - arc.circle.center)
                        }
                    }
                    .cap_magnitude(0.3 * time_step.as_secs_f32());

                    let orientation = match orientation_mode {
                        OrientationMode::AlignWithPath => {
                            if step.norm_squared() < f32::EPSILON {
                                UnitComplex::identity()
                            } else {
                                UnitComplex::from_cos_sin_unchecked(step.x, step.y)
                            }
                        }
                        OrientationMode::Override(orientation) => *orientation,
                    };

                    let previous_robot_to_field = *robot_to_field;

                    *robot_to_field = Isometry2::new(
                        robot_to_field.translation.vector + robot_to_field.rotation * step,
                        robot_to_field.rotation.angle()
                            + orientation.angle().clamp(
                                -std::f32::consts::FRAC_PI_4 * time_step.as_secs_f32(),
                                std::f32::consts::FRAC_PI_4 * time_step.as_secs_f32(),
                            ),
                    );

                    for obstacle in robot.database.main_outputs.obstacles.iter_mut() {
                        obstacle.position =
                            robot_to_field.inverse() * previous_robot_to_field * obstacle.position;
                    }

                    head
                }
                MotionCommand::InWalkKick {
                    head,
                    kick,
                    kicking_side,
                    strength,
                } => {
                    if let Some(ball) = self.ball.as_mut() {
                        let side = match kicking_side {
                            Side::Left => 1.0,
                            Side::Right => -1.0,
                        };

                        // TODO: Check if ball is even in range
                        // let kick_location = robot_to_field * ();
                        if (self.time_elapsed - robot.last_kick_time).as_secs_f32() > 1.0 {
                            let direction = match kick {
                                KickVariant::Forward => vector![1.0, 0.0],
                                KickVariant::Turn => vector![0.707, 0.707 * side],
                                KickVariant::Side => vector![0.0, 1.0 * -side],
                            };
                            ball.velocity += *robot_to_field * direction * *strength * 2.5;
                            robot.last_kick_time = self.time_elapsed;
                        };
                    }
                    head
                }
                MotionCommand::SitDown { head } => head,
                MotionCommand::Stand { head } => head,
                _ => &HeadMotion::Center,
            };

            let desired_head_yaw = match head_motion {
                HeadMotion::ZeroAngles => 0.0,
                HeadMotion::Center => 0.0,
                HeadMotion::LookAround | HeadMotion::SearchForLostBall => {
                    robot.database.main_outputs.look_around.yaw
                }
                HeadMotion::LookAt { target, .. } => target.coords.angle(&Vector2::x_axis()),
                HeadMotion::LookLeftAndRightOf { target } => {
                    let glance_factor = self.time_elapsed.as_secs_f32().sin();
                    target.coords.angle(&Vector2::x_axis())
                        + glance_factor * robot.parameters.look_at.glance_angle
                }
                HeadMotion::Unstiff => 0.0,
            };

            let max_head_rotation_per_cycle =
                robot.parameters.head_motion.maximum_velocity.yaw * time_step.as_secs_f32();
            let diff =
                desired_head_yaw - robot.database.main_outputs.sensor_data.positions.head.yaw;
            let movement = diff.clamp(-max_head_rotation_per_cycle, max_head_rotation_per_cycle);

            robot.database.main_outputs.sensor_data.positions.head.yaw += movement;
        }
    }

    fn cycle_robots(&mut self, now: std::time::SystemTime) -> Result<()> {
        let incoming_messages = take(&mut self.messages);

        for (player_number, robot) in self.robots.iter_mut() {
            let incoming_messages: Vec<_> = incoming_messages
                .iter()
                .filter_map(|(sender, message)| {
                    (sender != player_number).then_some(IncomingMessage::Spl(*message))
                })
                .collect();
            let messages_with_time =
                BTreeMap::from_iter([(now, incoming_messages.iter().collect())]);

            robot.database.main_outputs.cycle_time.start_time = now;

            let robot_to_field = robot
                .database
                .main_outputs
                .robot_to_field
                .expect("simulated robots should always have a known pose");
            let ball_visible = self.ball.as_ref().is_some_and(|ball| {
                let ball_in_ground = robot_to_field.inverse() * ball.position;
                let head_rotation = UnitComplex::from_angle(
                    robot.database.main_outputs.sensor_data.positions.head.yaw,
                );
                let ball_in_head = head_rotation.inverse() * ball_in_ground;
                let field_of_view = robot.field_of_view();
                let angle_to_ball = ball_in_head.coords.angle(&Vector2::x_axis());

                angle_to_ball.abs() < field_of_view / 2.0 && ball_in_head.coords.norm() < 3.0
            });
            if ball_visible {
                robot.ball_last_seen = Some(now);
            }
            robot.database.main_outputs.ball_position =
                if robot.ball_last_seen.is_some_and(|last_seen| {
                    now.duration_since(last_seen).expect("time ran backwards")
                        < robot.parameters.ball_filter.hypothesis_timeout
                }) {
                    self.ball.as_ref().map(|ball| BallPosition {
                        position: robot_to_field.inverse() * ball.position,
                        velocity: robot_to_field.inverse() * ball.velocity,
                        last_seen: now,
                    })
                } else {
                    None
                };
            robot.database.main_outputs.primary_state =
                match (robot.is_penalized, self.filtered_game_state) {
                    (true, _) => PrimaryState::Penalized,
                    (false, FilteredGameState::Initial) => PrimaryState::Initial,
                    (false, FilteredGameState::Ready { .. }) => PrimaryState::Ready,
                    (false, FilteredGameState::Set) => PrimaryState::Set,
                    (false, FilteredGameState::Playing { .. }) => PrimaryState::Playing,
                    (false, FilteredGameState::Finished) => PrimaryState::Finished,
                };
            robot.database.main_outputs.filtered_game_controller_state =
                Some(FilteredGameControllerState {
                    game_state: self.filtered_game_state,
                    ..Default::default()
                });
            robot.database.main_outputs.game_controller_state = Some(self.game_controller_state);

            robot.cycle(messages_with_time)?;

            for message in robot.interface.take_outgoing_messages() {
                if let OutgoingMessage::Spl(message) = message {
                    self.messages.push((*player_number, message));
                    self.game_controller_state.remaining_amount_of_messages -= 1
                }
            }
        }

        Ok(())
    }

    fn move_ball(&mut self, time_step: Duration) -> Vec<Event> {
        let mut events = Vec::new();
        if let Some(ball) = self.ball.as_mut() {
            ball.position += ball.velocity * time_step.as_secs_f32();
            ball.velocity *= 0.98;

            if ball.position.x.abs() > 4.5 && ball.position.y < 0.75 {
                events.push(Event::Goal);
            }
        }
        events
    }

    pub fn get_lua_state(&self) -> LuaState {
        LuaState {
            time_elapsed: self.time_elapsed.as_secs_f32(),
            cycle_count: self.cycle_count,
            // TODO: Expose robot data to lua again
            // robots: self.robots.iter().map(LuaRobot::new).collect(),
            robots: Default::default(),
            ball: self.ball.clone(),
            messages: self.messages.clone(),

            finished: self.finished,

            game_controller_state: self.game_controller_state,
            filtered_game_state: self.filtered_game_state,
        }
    }

    pub fn load_lua_state(&mut self, lua_state: LuaState) -> Result<()> {
        self.ball = lua_state.ball;
        self.cycle_count = lua_state.cycle_count;
        for lua_robot in lua_state.robots {
            let mut robot = Robot::try_new(lua_robot.parameters.player_number)
                .expect("Creating dummy robot should never fail");
            robot.database = lua_robot.database;
            robot.parameters = lua_robot.parameters;
            self.robots.insert(robot.parameters.player_number, robot);
        }

        self.finished = lua_state.finished;

        self.game_controller_state = lua_state.game_controller_state;
        self.filtered_game_state = lua_state.filtered_game_state;

        Ok(())
    }
}

impl Default for State {
    fn default() -> Self {
        let robots = HashMap::new();

        let game_controller_state = GameControllerState {
            game_state: GameState::Initial,
            game_phase: GamePhase::Normal,
            kicking_team: Team::Hulks,
            last_game_state_change: UNIX_EPOCH,
            penalties: Players {
                one: None,
                two: None,
                three: None,
                four: None,
                five: None,
                six: None,
                seven: None,
            },
            remaining_amount_of_messages: 1200,
            sub_state: None,
            hulks_team_is_home_after_coin_toss: false,
        };

        Self {
            time_elapsed: Duration::ZERO,
            cycle_count: 0,
            robots,
            ball: None,
            messages: Vec::new(),
            finished: false,
            game_controller_state,
            filtered_game_state: FilteredGameState::Initial,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct LuaState {
    pub time_elapsed: f32,
    pub cycle_count: usize,
    pub robots: Vec<LuaRobot>,
    pub ball: Option<Ball>,
    pub messages: Vec<(PlayerNumber, HulkMessage)>,
    pub finished: bool,
    pub game_controller_state: GameControllerState,
    pub filtered_game_state: FilteredGameState,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct LuaRobot {
    database: Database,
    parameters: Parameters,
}

impl LuaRobot {
    pub fn new(robot: &Robot) -> Self {
        Self {
            database: robot.database.clone(),
            parameters: robot.parameters.clone(),
        }
    }
}
