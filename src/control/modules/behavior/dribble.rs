use nalgebra::{point, vector, Isometry2, Point2, UnitComplex, Vector2};
use types::{
    direct_path, rotate_towards, FieldDimensions, HeadMotion, MotionCommand, OrientationMode,
    PathObstacle, WorldState,
};

use crate::framework::{configuration::DribblePose, AdditionalOutput};

use super::walk_to_pose::{hybrid_alignment, WalkPathPlanner};

pub fn execute(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    dribble_pose: &DribblePose,
    walk_path_planner: &WalkPathPlanner,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
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
    if relative_dribble_pose.translation.vector.x.abs() > dribble_pose.target_reached_thresholds.x
        || relative_dribble_pose.translation.vector.y.abs()
            > dribble_pose.target_reached_thresholds.y
        || relative_dribble_pose.rotation.angle().abs() > dribble_pose.target_reached_thresholds.z
    {
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
        let relative_dribble_pose = robot_to_field.inverse() * pose_behind_ball;
        let head = HeadMotion::LookAt {
            target: relative_ball_position,
        };
        let orientation_mode = hybrid_alignment(
            relative_dribble_pose,
            dribble_pose.hybrid_align_distance,
            dribble_pose.distance_to_be_aligned,
        );
        let path = walk_path_planner.plan(
            relative_dribble_pose * Point2::origin(),
            robot_to_field,
            path_obstacles_output,
        );
        let command = MotionCommand::Walk {
            head,
            orientation_mode,
            path,
        };
        Some(command)
    } else {
        Some(MotionCommand::Walk {
            head: HeadMotion::LookAt {
                target: relative_ball_position,
            },
            orientation_mode: OrientationMode::AlignWithPath,
            path: direct_path(Point2::origin(), point![1.0, 0.0]),
        })
    }
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
        rotate_towards(closest_position, opponent_goal).angle(),
    )
}
