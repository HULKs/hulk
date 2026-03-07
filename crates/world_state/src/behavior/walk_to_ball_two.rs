use coordinate_systems::Ground;
use framework::AdditionalOutput;
use linear_algebra::{Pose2, Vector2};
use types::{
    ball_position::BallPosition,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    parameters::WalkToPoseParameters,
    path_obstacles::PathObstacle,
    world_state::WorldState,
};

use crate::behavior::walk_to_pose::{WalkPathPlanner, WalkToPoseState};

pub fn execute(
    ball_position: Option<BallPosition<Ground>>,
    walk_to_pose_state: &mut WalkToPoseState,
    cycle_time: f32,
    parameters: &WalkToPoseParameters,
    walk_path_planner: &WalkPathPlanner,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    world_state: &WorldState,
) -> Option<MotionCommand> {
    match ball_position {
        Some(ball_position) => {
            let head = HeadMotion::LookAt {
                target: ball_position.position,
                image_region_target: ImageRegion::Center,
            };
            walk_to_pose_state.walk_to(
                Pose2::from(ball_position.position),
                cycle_time,
                head,
                parameters,
                walk_path_planner,
                &[],
                &[],
                path_obstacles_output,
                world_state,
            )
        }
        None => Some(MotionCommand::WalkWithVelocity {
            head: HeadMotion::Center {
                image_region_target: ImageRegion::Top,
            },
            velocity: Vector2::zeros(),
            angular_velocity: 0.0,
        }),
    }
}
