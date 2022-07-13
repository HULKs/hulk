use nalgebra::{Isometry2, Point2, UnitComplex};
use types::{
    direct_path, FieldDimensions, HeadMotion, MotionCommand, Obstacle, OrientationMode,
    PathObstacle, PathSegment, WorldState,
};

use crate::{
    control::{filtering::less_than_with_hysteresis, PathPlanner},
    framework::{configuration, AdditionalOutput},
};

pub struct WalkPathPlanner<'cycle> {
    field_dimensions: &'cycle FieldDimensions,
    configuration: &'cycle configuration::PathPlanning,
}

impl<'cycle> WalkPathPlanner<'cycle> {
    pub fn new(
        field_dimensions: &'cycle FieldDimensions,
        configuration: &'cycle configuration::PathPlanning,
    ) -> Self {
        Self {
            field_dimensions,
            configuration,
        }
    }
    pub fn plan(
        &self,
        target: Point2<f32>,
        robot_to_field: Isometry2<f32>,
        ball_position: Option<Point2<f32>>,
        obstacles: &[Obstacle],
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Vec<PathSegment> {
        let mut planner = PathPlanner::new();
        planner.with_obstacles(obstacles, self.configuration.robot_radius_at_hip_height);
        planner.with_field_borders(
            robot_to_field.inverse(),
            self.field_dimensions.length,
            self.field_dimensions.width,
            self.field_dimensions.border_strip_width,
        );
        if let Some(ball_position) = ball_position {
            planner.with_ball(
                ball_position,
                self.configuration.ball_obstacle_radius,
                self.configuration.robot_radius_at_foot_height,
            );
        }

        let path = planner.plan(Point2::origin(), target).unwrap();
        path_obstacles_output.fill_on_subscription(|| planner.obstacles.clone());
        path.unwrap_or_else(|| direct_path(Point2::origin(), Point2::origin()))
    }
}

pub struct WalkAndStand<'cycle> {
    world_state: &'cycle WorldState,
    configuration: &'cycle configuration::WalkAndStand,
    walk_path_planner: &'cycle WalkPathPlanner<'cycle>,
    last_motion_command: &'cycle MotionCommand,
}

impl<'cycle> WalkAndStand<'cycle> {
    pub fn new(
        world_state: &'cycle WorldState,
        configuration: &'cycle configuration::WalkAndStand,
        walk_path_planner: &'cycle WalkPathPlanner,
        last_motion_command: &'cycle MotionCommand,
    ) -> Self {
        Self {
            world_state,
            configuration,
            walk_path_planner,
            last_motion_command,
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
        let was_standing_last_cycle =
            matches!(self.last_motion_command, MotionCommand::Stand { .. });
        let is_reached = less_than_with_hysteresis(
            was_standing_last_cycle,
            distance_to_walk,
            self.configuration.target_reached_thresholds.x,
            self.configuration.hysteresis.x,
        ) && less_than_with_hysteresis(
            was_standing_last_cycle,
            angle_to_walk.abs(),
            self.configuration.target_reached_thresholds.y,
            self.configuration.hysteresis.y,
        );
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
                    self.world_state.ball.map(|ball| ball.position),
                    &self.world_state.obstacles,
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
