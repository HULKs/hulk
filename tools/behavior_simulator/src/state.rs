use std::{
    collections::{BTreeMap, HashMap},
    iter::once,
    mem::take,
    time::{Duration, UNIX_EPOCH},
};

use color_eyre::Result;
use nalgebra::{vector, Isometry2, Point2, UnitComplex, Vector2};
use serde::{Deserialize, Serialize};
use spl_network_messages::{GamePhase, GameState, PlayerNumber, SplMessage, Team};
use structs::{control::AdditionalOutputs, Configuration};
use types::{
    messages::{IncomingMessage, OutgoingMessage},
    BallPosition, FilteredGameState, GameControllerState, KickVariant, LineSegment, MotionCommand,
    OrientationMode, PathSegment, Players, PrimaryState, Side,
};

use crate::{cycler::Database, robot::Robot};

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
    pub robots: HashMap<PlayerNumber, Robot>,
    pub ball: Option<Ball>,
    pub messages: Vec<(PlayerNumber, SplMessage)>,

    pub finished: bool,

    pub game_controller_state: GameControllerState,
    pub filtered_game_state: FilteredGameState,
}

impl State {
    pub fn new() -> Self {
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
            },
            remaining_amount_of_messages: 1200,
            set_play: None,
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

    pub fn cycle(&mut self, time_step: Duration) -> Result<Vec<Event>> {
        let now = UNIX_EPOCH + self.time_elapsed;

        let incoming_messages = take(&mut self.messages);

        let mut events = vec![Event::Cycle];

        for (player_number, robot) in self.robots.iter_mut() {
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
                        OrientationMode::AlignWithPath => {
                            if step.norm_squared() < f32::EPSILON {
                                UnitComplex::identity()
                            } else {
                                UnitComplex::from_cos_sin_unchecked(step.x, step.y)
                            }
                        }
                        OrientationMode::Override(orientation) => *orientation,
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
                            Side::Left => 1.0,
                            Side::Right => -1.0,
                        };

                        // TODO: Check if ball is even in range
                        // let kick_location = robot_to_field * ();

                        let strength = 1.0;
                        let direction = match kick {
                            KickVariant::Forward => vector![1.0, 0.0],
                            KickVariant::Turn => vector![0.707, 0.707 * side],
                            KickVariant::Side => vector![0.0, 1.0 * -side],
                        };
                        ball.velocity += *robot_to_field * direction * strength;
                    }
                }
                _ => {}
            }

            let incoming_messages: Vec<_> = incoming_messages
                .iter()
                .filter_map(|(sender, message)| {
                    (sender != player_number).then_some(IncomingMessage::Spl(*message))
                })
                .collect();
            robot.database.main_outputs.game_controller_state = Some(GameControllerState {
                game_state: GameState::Playing,
                game_phase: GamePhase::Normal,
                kicking_team: Team::Uncertain,
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
                match (robot.is_penalized, self.filtered_game_state) {
                    (true, _) => PrimaryState::Penalized,
                    (false, FilteredGameState::Initial) => PrimaryState::Initial,
                    (false, FilteredGameState::Ready { .. }) => PrimaryState::Ready,
                    (false, FilteredGameState::Set) => PrimaryState::Set,
                    (false, FilteredGameState::Playing { .. }) => PrimaryState::Playing,
                    (false, FilteredGameState::Finished) => PrimaryState::Finished,
                };
            robot.database.main_outputs.filtered_game_state = Some(self.filtered_game_state);
            robot.database.main_outputs.game_controller_state = Some(self.game_controller_state);

            robot.cycle(messages)?;

            for message in robot.interface.take_outgoing_messages() {
                if let OutgoingMessage::Spl(message) = message {
                    self.messages.push((*player_number, message));
                    self.game_controller_state.remaining_amount_of_messages -= 1
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

        Ok(events)
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
            let mut robot = Robot::try_new(lua_robot.configuration.player_number)
                .expect("Creating dummy robot should never fail");
            robot.database = lua_robot.database;
            robot.configuration = lua_robot.configuration;
            self.robots.insert(robot.configuration.player_number, robot);
        }

        self.finished = lua_state.finished;

        self.game_controller_state = lua_state.game_controller_state;
        self.filtered_game_state = lua_state.filtered_game_state;

        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
pub struct LuaState {
    pub time_elapsed: f32,
    pub cycle_count: usize,
    pub robots: Vec<LuaRobot>,
    pub ball: Option<Ball>,
    pub messages: Vec<(PlayerNumber, SplMessage)>,
    pub finished: bool,
    pub game_controller_state: GameControllerState,
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
