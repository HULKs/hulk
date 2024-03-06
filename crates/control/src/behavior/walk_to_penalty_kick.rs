use coordinate_systems::IntoTransform;
use framework::AdditionalOutput;
use nalgebra::Isometry2;
use types::{
    coordinate_systems::{Field, Ground},
    field_dimensions::FieldDimensions,
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
    field_dimensions: &FieldDimensions,
) -> Option<MotionCommand> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let kick_off_pose = Isometry2::translation(
        field_dimensions.length / 2.0
            - field_dimensions.penalty_marker_distance
            - field_dimensions.penalty_marker_size * 2.0,
        0.0,
    )
    .framed_transform::<Ground, Field>();
    walk_and_stand.execute(
        ground_to_field.inverse() * kick_off_pose,
        look_action.execute(),
        path_obstacles_output,
    )
}
