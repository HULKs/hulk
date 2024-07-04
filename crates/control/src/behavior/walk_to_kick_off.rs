use coordinate_systems::Field;
use framework::AdditionalOutput;
use linear_algebra::{Orientation2, Pose2};
use types::{
    motion_command::MotionCommand, parameters::RolePositionsParameters,
    path_obstacles::PathObstacle, world_state::WorldState,
};

use super::{head::LookAction, walk_to_pose::WalkAndStand};

pub fn execute(
    world_state: &WorldState,
    walk_and_stand: &WalkAndStand,
    look_action: &LookAction,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    role_positions: RolePositionsParameters,
) -> Option<MotionCommand> {
    let ground_to_field = world_state.robot.ground_to_field?;
    walk_and_stand.execute(
        ground_to_field.inverse()
            * Pose2::<Field>::from_parts(
                role_positions.striker_kickoff_position,
                Orientation2::new(role_positions.striker_kickoff_orientation),
            ),
        look_action.execute(),
        path_obstacles_output,
    )
}
