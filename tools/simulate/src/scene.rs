pub mod ball;
pub mod field;
pub mod goal;
pub mod object;
pub mod palette;
pub mod robot;
pub mod visual;

use bevy::prelude::*;

use crate::{bevy_mujoco::MujocoModelUpdateSet, parameters::SimulatorParameterSyncSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct SceneParameterUpdateSet;

pub struct ObjectsPlugin;

impl Plugin for ObjectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<visual::ObjectVisualAssets>()
            .configure_sets(
                PreUpdate,
                SceneParameterUpdateSet
                    .after(SimulatorParameterSyncSet)
                    .before(MujocoModelUpdateSet),
            )
            .add_systems(
                PreUpdate,
                (ball::update_ball_dimensions, goal::update_goal_dimensions)
                    .in_set(SceneParameterUpdateSet),
            )
            .add_systems(Update, object::cleanup_parts);
    }
}
