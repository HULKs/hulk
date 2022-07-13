use nalgebra::{point, Point2, UnitComplex};
use types::{direct_path, HeadMotion, MotionCommand, OrientationMode, WorldState};

pub fn execute(_world_state: &WorldState) -> Option<MotionCommand> {
    Some(MotionCommand::Walk {
        head: HeadMotion::SearchForLostBall,
        orientation_mode: OrientationMode::Override(UnitComplex::new(1.0)),
        path: direct_path(Point2::origin(), point![0.0, 0.0]),
    })
}
