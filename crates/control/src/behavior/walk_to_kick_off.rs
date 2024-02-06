use coordinate_systems::{Framed, IntoTransform};
use framework::AdditionalOutput;
use nalgebra::{Point2, Translation2};
use types::{
    coordinate_systems::{Field, Ground},
    motion_command::MotionCommand,
    path_obstacles::PathObstacle,
    world_state::WorldState,
};

use super::{head::LookAction, walk_to_pose::WalkAndStand};

pub fn execute(
    world_state: &WorldState,
    walk_and_stand: &WalkAndStand,
    look_action: &LookAction,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    striker_set_position: Framed<Field, Point2<f32>>,
) -> Option<MotionCommand> {
    let ground_to_field = world_state.robot.ground_to_field?;
    walk_and_stand.execute(
        (ground_to_field.inverse().inner * Translation2::from(striker_set_position.inner))
            .framed_transform::<Ground, Ground>(),
        look_action.execute(),
        path_obstacles_output,
    )
}
