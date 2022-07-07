use nalgebra::{Isometry2, Point2, UnitComplex};
use types::{
    direct_path, FieldDimensions, HeadMotion, MotionCommand, OrientationMode, PathObstacle,
    PathSegment, WorldState,
};

use crate::{
    control::PathPlanner,
    framework::{configuration, AdditionalOutput},
};

pub struct WalkPathPlanner<'cycle> {
    world_state: &'cycle WorldState,
    field_dimensions: &'cycle FieldDimensions,
    configuration: &'cycle configuration::PathPlanning,
}

impl<'cycle> WalkPathPlanner<'cycle> {
    pub fn new(
        world_state: &'cycle WorldState,
        field_dimensions: &'cycle FieldDimensions,
        configuration: &'cycle configuration::PathPlanning,
    ) -> Self {
        Self {
            world_state,
            field_dimensions,
            configuration,
        }
    }
    pub fn plan(
        &self,
        target: Point2<f32>,
        robot_to_field: Isometry2<f32>,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Vec<PathSegment> {
        let mut planner = PathPlanner::new(Point2::origin(), target)
            .with_obstacles(&self.world_state.obstacles, self.configuration.robot_radius)
            .with_field_borders(
                robot_to_field.inverse(),
                self.field_dimensions.length,
                self.field_dimensions.width,
                self.field_dimensions.border_strip_width,
            );

        let path = planner.plan().unwrap();
        path_obstacles_output.fill_on_subscription(|| planner.obstacles.clone());
        path.unwrap_or_else(|| direct_path(Point2::origin(), Point2::origin()))
    }
}

pub struct WalkAndStand<'cycle> {
    world_state: &'cycle WorldState,
    configuration: &'cycle configuration::WalkAndStand,
    walk_path_planner: &'cycle WalkPathPlanner<'cycle>,
}

impl<'cycle> WalkAndStand<'cycle> {
    pub fn new(
        world_state: &'cycle WorldState,
        configuration: &'cycle configuration::WalkAndStand,
        walk_path_planner: &'cycle WalkPathPlanner,
    ) -> Self {
        Self {
            world_state,
            configuration,
            walk_path_planner,
        }
    }

    pub fn execute(
        &self,
        target_pose: Isometry2<f32>,
        head: HeadMotion,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        let robot_to_field = self.world_state.robot.robot_to_field?;
        let distance_to_walk = target_pose.translation.vector.norm();
        let angle_to_walk = target_pose.rotation.angle();
        let is_reached = distance_to_walk < self.configuration.target_reached_thresholds.x
            && angle_to_walk.abs() < self.configuration.target_reached_thresholds.y;
        let orientation_mode = hybrid_alignment(
            target_pose,
            self.configuration.hybrid_align_distance,
            self.configuration.distance_to_be_aligned,
        );

        if is_reached {
            Some(MotionCommand::Stand { head })
        } else {
            Some(MotionCommand::Walk {
                head,
                orientation_mode,
                path: self.walk_path_planner.plan(
                    target_pose * Point2::origin(),
                    robot_to_field,
                    path_obstacles_output,
                ),
            })
        }
    }
}

pub fn hybrid_alignment(
    target_pose: Isometry2<f32>,
    hybrid_align_distance: f32,
    distance_to_be_aligned: f32,
) -> OrientationMode {
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
