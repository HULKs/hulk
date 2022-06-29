use nalgebra::{Isometry2, Point2, UnitComplex};

use crate::{
    control::{PathObstacle, PathPlanner},
    framework::{configuration, AdditionalOutput},
    types::{
        direct_path, FieldDimensions, HeadMotion, MotionCommand, Obstacle, OrientationMode,
        PathSegment, WorldState,
    },
};

pub fn walk_and_stand_with_head(
    target_pose: Isometry2<f32>,
    world_state: &WorldState,
    head: HeadMotion,
    field_dimensions: &FieldDimensions,
    walk_configuration: &configuration::WalkToPose,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
) -> Option<MotionCommand> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let obstacles = &world_state.obstacles;
    let distance_to_walk = target_pose.translation.vector.norm();
    let angle_to_walk = target_pose.rotation.angle();
    let is_reached = distance_to_walk < walk_configuration.target_reached_threshold.x
        && angle_to_walk.abs() < walk_configuration.target_reached_threshold.y;

    if is_reached {
        Some(MotionCommand::Stand { head })
    } else {
        Some(MotionCommand::Walk {
            head,
            orientation_mode: hybrid_alignment(target_pose, walk_configuration),
            path: path_to_target(
                target_pose * Point2::origin(),
                robot_to_field,
                obstacles,
                field_dimensions,
                walk_configuration,
                path_obstacles_output,
            ),
        })
    }
}

fn path_to_target(
    target: Point2<f32>,
    robot_to_field: Isometry2<f32>,
    obstacles: &[Obstacle],
    field_dimensions: &FieldDimensions,
    walk_configuration: &configuration::WalkToPose,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
) -> Vec<PathSegment> {
    let mut planner = PathPlanner::new(Point2::origin(), target)
        .with_obstacles(obstacles, walk_configuration.robot_radius)
        .with_field_borders(
            robot_to_field.inverse(),
            field_dimensions.length,
            field_dimensions.width,
            field_dimensions.border_strip_width - walk_configuration.robot_radius,
        );

    let path = planner.plan().unwrap();
    path_obstacles_output.fill_on_subscription(|| planner.obstacles.clone());
    path.unwrap_or_else(|| direct_path(Point2::origin(), Point2::origin()))
}

fn hybrid_alignment(
    target_pose: Isometry2<f32>,
    walk_configuration: &configuration::WalkToPose,
) -> OrientationMode {
    let hybrid_align_distance = walk_configuration.hybrid_align_distance;
    let distance_to_be_aligned = walk_configuration.distance_to_be_aligned;
    assert!(hybrid_align_distance > distance_to_be_aligned);
    let distance_to_target = target_pose.translation.vector.norm();
    if distance_to_target >= hybrid_align_distance {
        return OrientationMode::AlignWithPath;
    }
    let target_facing_rotation =
        UnitComplex::new(target_pose.translation.y.atan2(target_pose.translation.x));
    let t = ((distance_to_target - distance_to_be_aligned)
        / (hybrid_align_distance - distance_to_be_aligned))
        .clamp(0.0, 1.0);
    OrientationMode::Override(target_pose.rotation.slerp(&target_facing_rotation, t))
}
