pub mod cost_fields;
pub mod geometry;
pub mod step_plan;
pub mod traits;
pub mod utils;

pub const VARIABLES_PER_STEP: usize = 3;

#[cfg(test)]
pub mod test_utils {
    pub mod decompose;
    pub mod gradient_type;
    pub mod verify_gradient;

    use std::f32::consts::FRAC_PI_2;

    use geometry::{arc::Arc, circle::Circle, direction::Direction, line_segment::LineSegment};
    use linear_algebra::{point, Orientation2};
    use types::planned_path::{Path, PathSegment};

    pub fn test_path() -> Path {
        Path {
            segments: vec![
                PathSegment::LineSegment(LineSegment(point![0.0, 0.0], point![3.0, 0.0])),
                PathSegment::Arc(Arc {
                    circle: Circle {
                        center: point![3.0, 1.0],
                        radius: 1.0,
                    },
                    start: Orientation2::new(3.0 * FRAC_PI_2),
                    end: Orientation2::new(0.0),
                    direction: Direction::Counterclockwise,
                }),
                PathSegment::LineSegment(LineSegment(point![4.0, 1.0], point![4.0, 4.0])),
            ],
        }
    }
}
