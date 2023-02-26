use crate::cycler::Database;
use mlua::{Function, Lua, LuaSerdeExt};
use nalgebra::{vector, Isometry2, Point2, UnitComplex, Vector2};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use spl_network_messages::SplMessage;
use std::{
    collections::BTreeMap,
    fs::read_to_string,
    iter::once,
    mem::take,
    sync::Arc,
    time::{Duration, UNIX_EPOCH},
};
use structs::{control::AdditionalOutputs, Configuration};
use types::{
    messages::{IncomingMessage, OutgoingMessage},
    BallPosition, LineSegment, MotionCommand, PathSegment, PrimaryState,
};

use crate::robot::Robot;

const SERIALIZE_OPTIONS: mlua::SerializeOptions =
    mlua::SerializeOptions::new().serialize_none_to_null(false);

enum Event {
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
        }
    }

    fn cycle(&mut self, time_step: Duration) -> Vec<Event> {
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

            if let Some(ball) = &self.ball {
                robot.database.main_outputs.ball_position = Some(types::BallPosition {
                    position: robot_to_field.inverse() * ball.position,
                    last_seen: now,
                })
            }

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

    pub fn stiffen_robots(&mut self) {
        for robot in &mut self.robots {
            robot.database.main_outputs.primary_state = PrimaryState::Playing;
        }
    }

    fn get_lua_state(&self) -> LuaState {
        LuaState {
            time_elapsed: self.time_elapsed.as_secs_f32(),
            cycle_count: self.cycle_count,
            // robots: self.robots.iter().map(LuaRobot::new).collect(),
            robots: Default::default(),
            ball: self.ball.clone(),
            messages: self.messages.clone(),
        }
    }

    fn load_lua_state(&mut self, lua_state: LuaState) {
        self.ball = lua_state.ball;
        self.cycle_count = lua_state.cycle_count;
        while self.robots.len() < lua_state.robots.len() {
            self.robots.push(Robot::new(1));
        }
        for (robot, lua_robot) in self.robots.iter_mut().zip(lua_state.robots.into_iter()) {
            robot.database = lua_robot.database;
            robot.configuration = lua_robot.configuration;
        }
    }
}

#[derive(Deserialize, Serialize)]
struct LuaState {
    pub time_elapsed: f32,
    pub cycle_count: usize,
    pub robots: Vec<LuaRobot>,
    pub ball: Option<Ball>,
    pub messages: Vec<(usize, SplMessage)>,
}

#[derive(Clone, Deserialize, Serialize)]
struct LuaRobot {
    database: Database,
    configuration: Configuration,
}

impl LuaRobot {
    fn new(robot: &Robot) -> Self {
        Self {
            database: robot.database.clone(),
            configuration: robot.configuration.clone(),
        }
    }
}

pub struct Simulator {
    pub state: Arc<Mutex<State>>,
    lua: Lua,
}

impl Simulator {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(State::new()));

        let lua = Lua::new();
        let script_text = read_to_string("test.lua").unwrap();
        let script = lua.load(&script_text).set_name("test.lua").unwrap();

        let new_robot = lua
            .create_function(|lua, number: usize| {
                let robot = Robot::new(number);
                Ok(lua.to_value(&LuaRobot::new(&robot)))
            })
            .unwrap();
        lua.globals().set("new_robot", new_robot).unwrap();

        lua.globals()
            .set(
                "state",
                lua.to_value_with(&state.lock().get_lua_state(), SERIALIZE_OPTIONS)
                    .unwrap(),
            )
            .unwrap();

        script.exec().unwrap();

        state
            .lock()
            .load_lua_state(lua.from_value(lua.globals().get("state").unwrap()).unwrap());

        Self { state, lua }
    }

    pub fn cycle(&mut self) {
        let events = {
            let mut state = self.state.lock();
            state.cycle(Duration::from_millis(12))
        };

        self.lua
            .globals()
            .set(
                "state",
                self.lua
                    .to_value_with(&self.state.lock().get_lua_state(), SERIALIZE_OPTIONS)
                    .unwrap(),
            )
            .unwrap();

        for event in events {
            match event {
                Event::Cycle => {
                    if let Ok(on_cycle) = self.lua.globals().get::<_, Function>("on_cycle") {
                        on_cycle.call::<_, ()>(()).unwrap();
                    }
                }
                Event::Goal => {
                    if let Ok(on_goal) = self.lua.globals().get::<_, Function>("on_goal") {
                        on_goal.call::<_, ()>(()).unwrap();
                    }
                }
            }
        }

        self.state.lock().load_lua_state(
            self.lua
                .from_value(self.lua.globals().get("state").unwrap())
                .unwrap(),
        );
    }
}
