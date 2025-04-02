use bevy::prelude::*;
use tokio::runtime::{Builder, Runtime};

pub struct AsyncRuntimePlugin;

impl Plugin for AsyncRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AsyncRuntime::new());
    }
}

#[derive(Resource)]
pub struct AsyncRuntime {
    pub runtime: Runtime,
}

impl AsyncRuntime {
    pub fn new() -> Self {
        Self {
            runtime: Builder::new_multi_thread().enable_all().build().unwrap(),
        }
    }
}
