use crate::cycler::Database;
use nalgebra::{vector, Isometry2, Point2, UnitComplex, Vector2};
use serde::{Deserialize, Serialize};
use spl_network_messages::SplMessage;
use std::{
    collections::BTreeMap,
    iter::once,
    mem::take,
    time::{Duration, UNIX_EPOCH},
};
use structs::{control::AdditionalOutputs, Configuration};
use types::{
    messages::{IncomingMessage, OutgoingMessage},
    BallPosition, FilteredGameState, LineSegment, MotionCommand, PathSegment, PrimaryState,
};

use crate::robot::Robot;

pub enum Event {
    Cycle,
    Goal,
}

#[derive(Default, Clone, Deserialize, Serialize)]
pub struct Ball {
    pub position: Point2<f32>,
    pub velocity: Vector2<f32>,
}

pub struct State {
    pub time_elapsed: Duration,
    pub cycle_count: usize,
    pub robots: Vec<Robot>,
    pub ball: Option<Ball>,
    pub messages: Vec<(usize, SplMessage)>,

    pub filtered_game_state: FilteredGameState,
}

impl State {
    pub fn new() -> Self {
        let robots = Vec::new();

        Self {
            time_elapsed: Duration::ZERO,
            cycle_count: 0,
            robots,
            ball: None,
            messages: Vec::new(),
            filtered_game_state: FilteredGameState::Initial,
        }
    }

    pub fn cycle(&mut self, time_step: Duration) -> Vec<Event> {
        let now = UNIX_EPOCH + self.time_elapsed;

        let incoming_messages = take(&mut self.messages);

        let mut events = vec![Event::Cycle];

        for (index, robot) in self.robots.iter_mut().enumerate() {
            let robot_to_field = robot
                .database
                .main_outputs
                .robot_to_field
                .as_mut()
                .expect("Simulated robots should always have a known pose");

            robot.database.additional_outputs = AdditionalOutputs::default();
            match &robot.database.main_outputs.motion_command {
                MotionCommand::Walk {
                    path,
                    orientation_mode,
                    ..
                } => {
                    let step = match path[0] {
                        PathSegment::LineSegment(LineSegment(_start, end)) => end,
                        PathSegment::Arc(arc, _orientation) => arc.end,
                    }
                    .coords
                    .cap_magnitude(0.3 * time_step.as_secs_f32());
                    let orientation = match orientation_mode {
                        types::OrientationMode::AlignWithPath => {
                            if step.norm_squared() < f32::EPSILON {
                                UnitComplex::identity()
                            } else {
                                UnitComplex::from_cos_sin_unchecked(step.x, step.y)
                            }
                        }
                        types::OrientationMode::Override(orientation) => *orientation,
                    };

                    *robot_to_field = Isometry2::new(
                        robot_to_field.translation.vector + robot_to_field.rotation * step,
                        robot_to_field.rotation.angle()
                            + orientation.angle().clamp(
                                -std::f32::consts::FRAC_PI_4 * time_step.as_secs_f32(),
                                std::f32::consts::FRAC_PI_4 * time_step.as_secs_f32(),
                            ),
                    )
                }
                MotionCommand::InWalkKick {
                    head: _,
                    kick,
                    kicking_side,
                } => {
                    if let Some(ball) = self.ball.as_mut() {
                        let side = match kicking_side {
                            types::Side::Left => 1.0,
                            types::Side::Right => -1.0,
                        };

                        // TODO: Check if ball is even in range
                        // let kick_location = robot_to_field * ();

                        let strength = 1.0;
                        let direction = match kick {
                            types::KickVariant::Forward => vector![1.0, 0.0],
                            types::KickVariant::Turn => vector![0.707, 0.707 * side],
                            types::KickVariant::Side => vector![0.0, 1.0 * -side],
                        };
                        ball.velocity += *robot_to_field * direction * strength;
                    }
                }
                _ => {}
            }

            let incoming_messages: Vec<_> = incoming_messages
                .iter()
                .filter_map(|(sender, message)| {
                    (*sender != index).then_some(IncomingMessage::Spl(*message))
                })
                .collect();
            robot.database.main_outputs.game_controller_state = Some(types::GameControllerState {
                game_state: spl_network_messages::GameState::Playing,
                game_phase: spl_network_messages::GamePhase::Normal,
                kicking_team: spl_network_messages::Team::Uncertain,
                last_game_state_change: now,
                penalties: Default::default(),
                remaining_amount_of_messages: 1200,
                set_play: None,
            });
            let messages = incoming_messages.iter().collect();
            let messages = BTreeMap::from_iter(once((now, messages)));

            robot.database.main_outputs.cycle_time.start_time = now;

            robot.database.main_outputs.ball_position =
                self.ball.as_ref().map(|ball| BallPosition {
                    position: robot_to_field.inverse() * ball.position,
                    last_seen: now,
                });

            robot.database.main_outputs.primary_state =
                match (robot.penalized, self.filtered_game_state) {
                    (true, _) => PrimaryState::Penalized,
                    (false, FilteredGameState::Initial) => PrimaryState::Initial,
                    (false, FilteredGameState::Ready { .. }) => PrimaryState::Ready,
                    (false, FilteredGameState::Set) => PrimaryState::Set,
                    (false, FilteredGameState::Playing { .. }) => PrimaryState::Playing,
                    (false, FilteredGameState::Finished) => PrimaryState::Finished,
                };

            robot.cycle(messages).unwrap();

            for message in robot.interface.take_outgoing_messages() {
                if let OutgoingMessage::Spl(message) = message {
                    self.messages.push((index, message));
                }
            }
        }

        if let Some(ball) = self.ball.as_mut() {
            ball.position += ball.velocity * time_step.as_secs_f32();
            ball.velocity *= 0.98;

            if ball.position.x.abs() > 4.5 && ball.position.y < 0.75 {
                events.push(Event::Goal);
            }
        }

        self.time_elapsed += time_step;
        self.cycle_count += 1;

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

            filtered_game_state: self.filtered_game_state,
        }
    }

    pub fn load_lua_state(&mut self, lua_state: LuaState) {
        self.ball = lua_state.ball;
        self.cycle_count = lua_state.cycle_count;
        while self.robots.len() < lua_state.robots.len() {
            self.robots.push(Robot::new(1));
        }
        for (robot, lua_robot) in self.robots.iter_mut().zip(lua_state.robots.into_iter()) {
            robot.database = lua_robot.database;
            robot.configuration = lua_robot.configuration;
        }

        self.filtered_game_state = lua_state.filtered_game_state;
    }
}

#[derive(Deserialize, Serialize)]
pub struct LuaState {
    pub time_elapsed: f32,
    pub cycle_count: usize,
    pub robots: Vec<LuaRobot>,
    pub ball: Option<Ball>,
    pub messages: Vec<(usize, SplMessage)>,

    pub filtered_game_state: FilteredGameState,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct LuaRobot {
    database: Database,
    configuration: Configuration,
}

impl LuaRobot {
    pub fn new(robot: &Robot) -> Self {
        Self {
            database: robot.database.clone(),
            configuration: robot.configuration.clone(),
        }
    }
}
