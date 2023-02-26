use std::{fs::read_to_string, path::Path, sync::Arc, time::Duration};

use mlua::{Function, Lua, LuaSerdeExt};
use parking_lot::Mutex;

use crate::{
    robot::Robot,
    state::{Event, LuaRobot, State},
};

const SERIALIZE_OPTIONS: mlua::SerializeOptions =
    mlua::SerializeOptions::new().serialize_none_to_null(false);

pub struct Simulator {
    pub state: Arc<Mutex<State>>,
    lua: Lua,
}

impl Simulator {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(State::new()));

        let lua = Lua::new();

        let new_robot = lua
            .create_function(|lua, number: usize| {
                let robot = Robot::new(number);
                Ok(lua.to_value(&LuaRobot::new(&robot)))
            })
            .unwrap();
        lua.globals().set("new_robot", new_robot).unwrap();

        Self { state, lua }
    }

    pub fn execute_script(&mut self, file_name: impl AsRef<Path>) {
        self.serialze_state();

        let script_text = read_to_string(&file_name).unwrap();
        let script = self
            .lua
            .load(&script_text)
            .set_name(file_name.as_ref().file_name().unwrap().to_str().unwrap())
            .unwrap();
        script.exec().unwrap();

        self.state.lock().load_lua_state(
            self.lua
                .from_value(self.lua.globals().get("state").unwrap())
                .unwrap(),
        );
    }

    pub fn cycle(&mut self) {
        let events = {
            let mut state = self.state.lock();
            state.cycle(Duration::from_millis(12))
        };

        self.serialze_state();

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

        self.deserialize_state();
    }

    fn serialze_state(&mut self) {
        self.lua
            .globals()
            .set(
                "state",
                self.lua
                    .to_value_with(&self.state.lock().get_lua_state(), SERIALIZE_OPTIONS)
                    .unwrap(),
            )
            .unwrap();
    }

    fn deserialize_state(&mut self) {
        self.state.lock().load_lua_state(
            self.lua
                .from_value(self.lua.globals().get("state").unwrap())
                .unwrap(),
        );
    }
}
