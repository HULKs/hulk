use std::env::current_dir;

use bevy::{
    app::{App, AppExit, First, Plugin, Update},
    core::{FrameCountPlugin, TaskPoolPlugin, TypeRegistrationPlugin},
    ecs::{
        event::{Events, ManualEventReader},
        schedule::IntoSystemConfigs,
    },
    time::Time,
};
use color_eyre::{
    eyre::{eyre, Context, ContextCompat},
    Result,
};
use repository::Repository;
use types::hardware::Ids;

use crate::{
    autoref::{autoref, autoref_plugin},
    ball::{move_ball, BallResource},
    field_dimensions::SimulatorFieldDimensions,
    game_controller::{game_controller_plugin, GameController},
    recorder::Recording,
    robot::{cycle_robots, move_robots, Messages},
    server::Parameters,
    time::{update_time, Ticks},
    whistle::WhistleResource,
};

#[derive(Default, Copy, Clone)]
pub struct SimulatorPlugin {
    pub use_recording: bool,
}

impl SimulatorPlugin {
    pub fn with_recording(mut self, use_recording: bool) -> Self {
        self.use_recording = use_recording;

        self
    }
}

impl Plugin for SimulatorPlugin {
    fn build(&self, app: &mut App) {
        let parameters = load_parameters().expect("failed to load parameters");

        app.add_plugins((
            TaskPoolPlugin::default(),
            TypeRegistrationPlugin,
            FrameCountPlugin,
        ))
        .add_plugins(autoref_plugin)
        .add_plugins(game_controller_plugin)
        .insert_resource(SimulatorFieldDimensions::from(parameters.field_dimensions))
        .insert_resource(GameController::default())
        .insert_resource(BallResource::default())
        .insert_resource(WhistleResource::default())
        .insert_resource(Messages::default())
        .insert_resource(Time::<()>::default())
        .insert_resource(Time::<Ticks>::default())
        .add_systems(First, update_time)
        .add_systems(Update, cycle_robots.before(move_robots).after(autoref))
        .add_systems(Update, move_robots)
        .add_systems(Update, move_ball.after(move_robots));

        if self.use_recording {
            app.add_plugins(crate::recorder::recording_plugin);
        }
    }
}

pub trait AppExt {
    fn run_to_completion(&mut self) -> Result<()>;
}

impl AppExt for App {
    fn run_to_completion(&mut self) -> Result<()> {
        let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();
        let exit = loop {
            self.update();
            if let Some(exit) = self
                .world_mut()
                .get_resource_mut::<Events<AppExit>>()
                .and_then(|events| app_exit_event_reader.read(&events).last().cloned())
            {
                break exit;
            }
        };
        if let Some(mut recording) = self.world_mut().get_resource_mut::<Recording>() {
            println!("serving {} frames", recording.frames.len());
            recording.serve()?
        }

        match exit {
            AppExit::Success => Ok(()),
            AppExit::Error(code) => Err(eyre!("Scenario exited with error code {code}")),
        }
    }
}

fn load_parameters() -> Result<Parameters> {
    let ids = Ids {
        body_id: "behavior_simulator".to_string(),
        head_id: "behavior_simulator".to_string(),
    };
    let current_directory = current_dir().wrap_err("failed to get current directory")?;
    let repository =
        Repository::find_root(current_directory).wrap_err("failed to get repository root")?;
    let parameters_path = repository.root.join("crates/bevyhavior_simulator");

    parameters::directory::deserialize(parameters_path, &ids, true)
        .wrap_err("failed to parse initial parameters")
}
