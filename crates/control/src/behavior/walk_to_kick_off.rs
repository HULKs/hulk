use coordinate_systems::Field;
use framework::AdditionalOutput;
use linear_algebra::Pose2;
use types::{
    motion_command::{MotionCommand, WalkSpeed},
    path_obstacles::PathObstacle,
    world_state::WorldState,
};

use super::{head::LookAction, walk_to_pose::WalkAndStand};

pub fn execute(
    world_state: &WorldState,
    walk_and_stand: &WalkAndStand,
    look_action: &LookAction,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    striker_kickoff_pose: Pose2<Field>,
    walk_speed: WalkSpeed,
) -> Option<MotionCommand> {
    let ground_to_field = world_state.robot.ground_to_field?;
    walk_and_stand.execute(
        ground_to_field.inverse() * striker_kickoff_pose,
        look_action.execute(),
        path_obstacles_output,
        walk_speed,
    )
}
