use nalgebra::{point, Point2, UnitComplex};
use types::{direct_path, HeadMotion, MotionCommand, OrientationMode, WorldState};

use super::walk_to_pose::WalkPathPlanner;

pub fn execute(
    _world_state: &WorldState,
    walk_path_planner: &WalkPathPlanner,
) -> Option<MotionCommand> {
    let head = HeadMotion::SearchForLostBall;
    let orientation_mode = OrientationMode::Override(UnitComplex::new(1.0));
    let path = direct_path(Point2::origin(), point![0.0, 0.0]);
    Some(walk_path_planner.walk_with_obstacle_avoiding_arms(head, orientation_mode, path))
}
