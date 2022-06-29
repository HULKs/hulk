use nalgebra::{point, Point2, UnitComplex};

use crate::types::{direct_path, MotionCommand, OrientationMode, WorldState};

use super::head::look_for_ball;

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    Some(MotionCommand::Walk {
        head: look_for_ball(world_state.ball),
        orientation_mode: OrientationMode::Override(UnitComplex::new(1.0)),
        path: direct_path(Point2::origin(), point![0.0, 0.0]),
    })
}
