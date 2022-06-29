use nalgebra::{point, vector, Isometry2, Point2, UnitComplex, Vector2};

use crate::{
    framework::configuration::DribblePose,
    types::{direct_path, FieldDimensions, HeadMotion, MotionCommand, OrientationMode, WorldState},
};

pub fn execute(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    dribble_pose: &DribblePose,
) -> Option<MotionCommand> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let relative_ball_position = world_state.ball?.position;
    let absolute_ball_position = robot_to_field * relative_ball_position;
    let pose_behind_ball = get_dribble_pose(
        field_dimensions,
        absolute_ball_position,
        robot_to_field,
        dribble_pose,
    );
    let relative_dribble_pose = robot_to_field.inverse() * pose_behind_ball;
    if relative_dribble_pose.translation.vector.y.abs() > 0.05
        || relative_dribble_pose.translation.vector.x.abs() > 0.15
        || relative_dribble_pose.rotation.angle().abs() > 0.05
    {
        return None;
    }
    Some(MotionCommand::Walk {
        head: HeadMotion::LookAt {
            target: relative_ball_position,
        },
        orientation_mode: OrientationMode::AlignWithPath,
        path: direct_path(Point2::origin(), point![1.0, 0.0]),
    })
}

pub fn get_dribble_pose(
    field_dimensions: &FieldDimensions,
    absolute_ball_position: Point2<f32>,
    robot_to_field: Isometry2<f32>,
    dribble_pose: &DribblePose,
) -> Isometry2<f32> {
    let opponent_goal = point![field_dimensions.length / 2.0 + 0.2, 0.0];
    let ball_to_goal = opponent_goal - absolute_ball_position;

    let left_position = dribble_pose.offset;
    let right_position = dribble_pose.offset.component_mul(&vector![1.0, -1.0]);

    let rotation_towards_goal = UnitComplex::rotation_between(&Vector2::x(), &ball_to_goal);
    let absolute_left_position = absolute_ball_position + rotation_towards_goal * left_position;
    let absolute_right_position = absolute_ball_position + rotation_towards_goal * right_position;

    let distance_to_left = (robot_to_field.inverse() * absolute_left_position)
        .coords
        .norm();
    let distance_to_right = (robot_to_field.inverse() * absolute_right_position)
        .coords
        .norm();

    let closest_position = if distance_to_left < distance_to_right {
        absolute_left_position
    } else {
        absolute_right_position
    };
    Isometry2::new(
        closest_position.coords,
        face_towards(closest_position, opponent_goal).angle(),
    )
}

fn face_towards(origin: Point2<f32>, target: Point2<f32>) -> UnitComplex<f32> {
    let origin_to_target = target - origin;
    UnitComplex::rotation_between(&Vector2::x(), &origin_to_target)
}
