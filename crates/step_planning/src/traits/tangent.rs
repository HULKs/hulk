use geometry::{circle::Circle, direction::Direction};
use linear_algebra::{vector, Orientation2, Vector2};

pub trait Tangent<Frame> {
    fn tangent(&self, angle: Orientation2<Frame>, direction: Direction) -> Vector2<Frame>;
}

impl<Frame> Tangent<Frame> for Circle<Frame> {
    fn tangent(&self, angle: Orientation2<Frame>, direction: Direction) -> Vector2<Frame> {
        let radius = angle.as_unit_vector();

        match direction {
            Direction::Clockwise => {
                vector![radius.y(), -radius.x()]
            }
            Direction::Counterclockwise => {
                vector![-radius.y(), radius.x()]
            }
            Direction::Collinear => Vector2::zeros(),
        }
    }
}
