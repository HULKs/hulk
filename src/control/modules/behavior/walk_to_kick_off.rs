use types::{FieldDimensions, MotionCommand, PathObstacle, WorldState};

use crate::framework::{configuration::DribblePose, AdditionalOutput};

use super::{dribble::get_dribble_pose, walk_to_pose::WalkAndStand};

pub fn execute(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    dribble_pose: &DribblePose,
    walk_and_stand: &WalkAndStand,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
) -> Option<MotionCommand> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let absolute_ball_position = world_state
        .ball
        .map(|ball| robot_to_field * ball.position)
        .unwrap_or_default();
    let pose_behind_ball = get_dribble_pose(
        field_dimensions,
        absolute_ball_position,
        robot_to_field,
        dribble_pose,
    );
    let head = types::HeadMotion::LookAround;
    walk_and_stand.execute(
        robot_to_field.inverse() * pose_behind_ball,
        head,
        path_obstacles_output,
    )
}
