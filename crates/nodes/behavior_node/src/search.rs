use coordinate_systems::Field;
use linear_algebra::{Orientation2, Point2, Pose2, distance, point, vector};
use types::field_dimensions::FieldDimensions;
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, MotionCommand, OrientationMode},
};

use crate::{node::Blackboard, walk::walk_to};

pub fn has_suggested_search_position(blackboard: &mut Blackboard) -> bool {
    blackboard.world_state.suggested_search_position.is_some()
}

pub fn leuchtturm(blackboard: &mut Blackboard) -> Status {
    let angular_velocity = get_leuchtturm_direction(blackboard);

    blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
        velocity: vector!(0.0, 0.0),
        angular_velocity,
    });
    Status::Success
}

fn get_leuchtturm_direction(blackboard: &Blackboard) -> f32 {
    if let MotionCommand::WalkWithVelocity {
        angular_velocity, ..
    } = blackboard.last_motion_command
        && angular_velocity.abs() > f32::EPSILON
    {
        return angular_velocity.signum();
    }

    if let (Some(last_ball), Some(ground_to_field)) = (
        &blackboard.last_ball,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_in_ground = ground_to_field.inverse() * last_ball.position;

        if ball_in_ground.y() < 0.0 {
            return -1.0;
        }
    }

    1.0
}

fn search_cell_target_pose(
    cell_center: Point2<Field>,
    robot_position: Point2<Field>,
    field_dimensions: FieldDimensions,
) -> Pose2<Field> {
    let min_x = (cell_center.x() - 0.5).max(-field_dimensions.length / 2.0);
    let max_x = (cell_center.x() + 0.5).min(field_dimensions.length / 2.0);
    let min_y = (cell_center.y() - 0.5).max(-field_dimensions.width / 2.0);
    let max_y = (cell_center.y() + 0.5).min(field_dimensions.width / 2.0);

    let corners = [
        point![min_x, min_y],
        point![min_x, max_y],
        point![max_x, min_y],
        point![max_x, max_y],
    ];
    let nearest_corner = corners
        .into_iter()
        .min_by(|a, b| distance(*a, robot_position).total_cmp(&distance(*b, robot_position)))
        .unwrap_or(cell_center);
    let direction_into_cell = cell_center.coords() - nearest_corner.coords();
    let orientation = if direction_into_cell.norm() > f32::EPSILON {
        Orientation2::from_vector(direction_into_cell)
    } else {
        Orientation2::new(0.0)
    };

    Pose2::from_parts(nearest_corner, orientation)
}

pub fn walk_to_search_position(blackboard: &mut Blackboard) -> Status {
    if let (Some(search_position), Some(ground_to_field)) = (
        blackboard.world_state.suggested_search_position,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let field_to_ground = ground_to_field.inverse();
        let robot_position = ground_to_field.as_pose().position();
        let target_pose_in_field =
            search_cell_target_pose(search_position, robot_position, blackboard.field_dimensions);
        let search_position_in_ground = field_to_ground * search_position;

        return walk_to(
            blackboard,
            field_to_ground * target_pose_in_field,
            blackboard.parameters.walk_speed.search,
            OrientationMode::LookAt {
                target: search_position_in_ground,
                tolerance: blackboard.parameters.walk_and_stand.orientation_tolerance,
            },
            blackboard
                .parameters
                .walk_and_stand
                .normal_distance_to_be_aligned,
            blackboard.parameters.walk_and_stand.hysteresis,
        );
    }

    Status::Failure
}
