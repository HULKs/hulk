use std::{fs::read_to_string, path::Path, sync::Arc, time::Duration};

use crate::{cycler::Database, robot::to_player_number, state::Ball};
use color_eyre::{
    eyre::{eyre, WrapErr},
    Result,
};
use mlua::{Error as LuaError, Function, Lua, LuaSerdeExt, SerializeOptions, Value};
use nalgebra::{Isometry2, Point2, Vector2};
use parking_lot::Mutex;
use types::{Obstacle, Players};

use crate::{
    robot::Robot,
    state::{Event, LuaRobot, State},
};

const SERIALIZE_OPTIONS: SerializeOptions = SerializeOptions::new().serialize_none_to_null(false);

pub struct Frame {
    pub ball: Option<Ball>,
    pub robots: Players<Option<Database>>,
}

pub struct Simulator {
    pub state: Arc<Mutex<State>>,
    lua: Lua,
}

impl Simulator {
    pub fn try_new() -> Result<Self> {
        let state = Arc::new(Mutex::new(State::new()));

        let lua = Lua::new();
        let create_robot = lua
            .create_function(|lua, player_number: usize| {
                let player_number = to_player_number(player_number).map_err(LuaError::external)?;
                let robot = Robot::try_new(player_number).map_err(LuaError::external)?;
                Ok(lua.to_value(&LuaRobot::new(&robot)))
            })
            .wrap_err("failed to create function create_robot")?;
        lua.globals()
            .set("create_robot", create_robot)
            .wrap_err("failed to insert create_robot")?;
        let error = lua
            .create_function(|_lua, message: String| -> Result<(), LuaError> {
                Err(LuaError::external(message))
            })
            .wrap_err("failed to create function create_robot")?;
        lua.globals()
            .set("error", error)
            .wrap_err("failed to insert create_robot")?;

        Ok(Self { state, lua })
    }

    pub fn execute_script(&mut self, file_name: impl AsRef<Path>) -> Result<()> {
        self.serialze_state()?;

        let script_text = read_to_string(&file_name)?;
        let script = self.lua.load(&script_text).set_name(
            file_name
                .as_ref()
                .file_name()
                .ok_or_else(|| eyre!("path contains no filename"))?
                .to_str()
                .ok_or_else(|| eyre!("filename is not valid unicode"))?,
        )?;
        script
            .exec()
            .wrap_err("failed to execute scenario script")?;

        self.deserialize_state()
    }

    pub fn run(&mut self) -> Result<Vec<Frame>> {
        let mut frames = Vec::new();
        loop {
            self.cycle()?;

            let state = self.state.lock();
            let mut robots = Players::<Option<Database>>::default();
            for (player_number, robot) in &state.robots {
                robots[*player_number] = Some(robot.database.clone())
            }
            frames.push(Frame {
                robots,
                ball: state.ball.clone(),
            });

            if state.finished {
                break;
            }
        }

        Ok(frames)
    }

    pub fn cycle(&mut self) -> Result<()> {
        let events = {
            let mut state = self.state.lock();
            state.cycle(Duration::from_millis(12))?
        };

        self.serialze_state()?;

        self.lua.scope(|scope| {
            self.lua.globals().set(
                "penalize",
                scope.create_function(|_, player_number: usize| {
                    let player_number =
                        to_player_number(player_number).map_err(LuaError::external)?;
                    self.state
                        .lock()
                        .robots
                        .get_mut(&player_number)
                        .unwrap()
                        .is_penalized = true;

                    Ok(())
                })?,
            )?;
            self.lua.globals().set(
                "unpenalize",
                scope.create_function(|_, player_number: usize| {
                    let player_number =
                        to_player_number(player_number).map_err(LuaError::external)?;
                    self.state
                        .lock()
                        .robots
                        .get_mut(&player_number)
                        .unwrap()
                        .is_penalized = false;

                    Ok(())
                })?,
            )?;

            self.lua.globals().set(
                "set_robot_pose",
                scope.create_function(
                    |lua, (player_number, position, angle): (usize, Value, f32)| {
                        let player_number =
                            to_player_number(player_number).map_err(LuaError::external)?;
                        let position: Vector2<f32> = lua.from_value(position)?;

                        self.state
                            .lock()
                            .robots
                            .get_mut(&player_number)
                            .unwrap()
                            .database
                            .main_outputs
                            .robot_to_field = Some(Isometry2::new(position, angle));

                        Ok(())
                    },
                )?,
            )?;

            self.lua.globals().set(
                "create_obstacle",
                scope.create_function(
                    |lua, (player_number, position, radius): (usize, Value, f32)| {
                        let player_number =
                            to_player_number(player_number).map_err(LuaError::external)?;
                        let position: Point2<f32> = lua.from_value(position)?;

                        let robot_to_field = self
                            .state
                            .lock()
                            .robots
                            .get(&player_number)
                            .unwrap()
                            .database
                            .main_outputs
                            .robot_to_field
                            .expect("simulated robots should always have a known pose");

                        self.state
                            .lock()
                            .robots
                            .get_mut(&player_number)
                            .unwrap()
                            .database
                            .main_outputs
                            .obstacles
                            .push(Obstacle::robot(
                                robot_to_field.inverse() * position,
                                radius,
                                radius,
                            ));

                        Ok(())
                    },
                )?,
            )?;

            self.lua.globals().set(
                "clear_obstacles",
                scope.create_function(|_, player_number: usize| {
                    let player_number =
                        to_player_number(player_number).map_err(LuaError::external)?;

                    self.state
                        .lock()
                        .robots
                        .get_mut(&player_number)
                        .unwrap()
                        .database
                        .main_outputs
                        .obstacles
                        .clear();

                    Ok(())
                })?,
            )?;

            for event in events {
                match event {
                    Event::Cycle => self.execute_event_callback("on_cycle")?,
                    Event::Goal => self.execute_event_callback("on_goal")?,
                }
            }

            Ok(())
        })?;

        self.deserialize_state()
    }

    fn execute_event_callback(&self, name: &str) -> Result<(), LuaError> {
        if let Ok(on_goal) = self.lua.globals().get::<_, Function>(name) {
            on_goal.call(())?;
        }

        Ok(())
    }

    fn serialze_state(&mut self) -> Result<()> {
        let lua_state = self.state.lock().get_lua_state();
        let value = self
            .lua
            .to_value_with(&lua_state, SERIALIZE_OPTIONS)
            .wrap_err("failed to serialize lua state")?;
        self.lua
            .globals()
            .set("state", value)
            .wrap_err("failed to set state in lua globals")
    }

    fn deserialize_state(&mut self) -> Result<()> {
        let value = self
            .lua
            .globals()
            .get("state")
            .wrap_err("failed to retrieve state from lua")?;
        let lua_state = self
            .lua
            .from_value(value)
            .wrap_err("failed to deserialize state")?;
        self.state
            .lock()
            .load_lua_state(lua_state)
            .wrap_err("failed to load lua state")
    }
}
