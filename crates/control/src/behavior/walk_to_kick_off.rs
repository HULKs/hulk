use coordinate_systems::Field;
use framework::AdditionalOutput;
use linear_algebra::{Point2, Pose2, Rotation2};
use types::{
    motion_command::{MotionCommand, OrientationMode, WalkSpeed},
    path_obstacles::PathObstacle,
    world_state::WorldState,
};

use super::{head::LookAction, walk_to_pose::WalkAndStand};

#[allow(clippy::too_many_arguments)]
pub fn execute(
    world_state: &WorldState,
    walk_and_stand: &WalkAndStand,
    look_action: &LookAction,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    kickoff_position: Point2<Field>,
    kick_off_angle: f32,
    walk_speed: WalkSpeed,
    distance_to_be_aligned: f32,
) -> Option<MotionCommand> {
    let kick_off_pose =
        Rotation2::<Field, Field>::new(-kick_off_angle) * Pose2::from(kickoff_position);
    let field_to_ground = world_state.robot.ground_to_field?.inverse();

    walk_and_stand.execute(
        field_to_ground * kick_off_pose,
        look_action.execute(),
        path_obstacles_output,
        walk_speed,
        OrientationMode::AlignWithPath,
        distance_to_be_aligned,
        walk_and_stand.parameters.hysteresis,
    )
}
