use nalgebra::Isometry2;

use crate::{
    framework::configuration::DribblePose,
    types::{FieldDimensions, WorldState},
};

use super::dribble::get_dribble_pose;

pub fn walk_behind_ball_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    dribble_pose: &DribblePose,
) -> Option<Isometry2<f32>> {
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
    Some(robot_to_field.inverse() * pose_behind_ball)
}
