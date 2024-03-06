use std::f32::consts::TAU;

use approx::{AbsDiffEq, RelativeEq};
use serde::{Deserialize, Serialize};

use linear_algebra::Point2;
use serialize_hierarchy::SerializeHierarchy;

use crate::{circle::Circle, direction::Direction};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct Arc<Frame> {
    pub circle: Circle<Frame>,
    pub start: Point2<Frame>,
    pub end: Point2<Frame>,
}

impl<Frame> AbsDiffEq for Arc<Frame>
where
    Frame: AbsDiffEq,
{
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        Self::Epsilon::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.circle.abs_diff_eq(&other.circle, epsilon)
            && self.start.abs_diff_eq(&other.start, epsilon)
            && self.end.abs_diff_eq(&other.end, epsilon)
    }
}

impl<Frame> RelativeEq for Arc<Frame>
where
    Frame: RelativeEq,
{
    fn default_max_relative() -> f32 {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.circle
            .relative_eq(&other.circle, epsilon, max_relative)
            && self.start.relative_eq(&other.start, epsilon, max_relative)
            && self.end.relative_eq(&other.end, epsilon, max_relative)
    }
}

impl<Frame> Arc<Frame> {
    pub fn new(circle: Circle<Frame>, start: Point2<Frame>, end: Point2<Frame>) -> Self {
        Self { circle, start, end }
    }

    pub fn length(&self, orientation: Direction) -> f32 {
        let vector_start = self.start - self.circle.center;
        let vector_end = self.end - self.circle.center;

        let angle_x_axis_to_start = vector_start.y().atan2(vector_start.x());
        let mut angle = vector_end.y().atan2(vector_end.x()) - angle_x_axis_to_start;

        if (orientation == Direction::Clockwise) && (angle > 0.0) {
            angle -= TAU;
            angle *= -1.0;
        }
        if (orientation == Direction::Counterclockwise) && (angle < 0.0) {
            angle += TAU;
        }
        (angle * self.circle.radius).abs()
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use approx::assert_relative_eq;
    use linear_algebra::{point, vector, UnitComplex};

    use super::*;

    struct SomeFrame;

    #[test]
    fn arc_cost_90_degrees() {
        let arc = Arc::<SomeFrame> {
            circle: Circle {
                center: point![1.0, 1.0],
                radius: 2.0,
            },
            start: point![1.0, 2.0],
            end: point![2.0, 1.0],
        };
        assert_relative_eq!(arc.length(Direction::Clockwise), PI);
        assert_relative_eq!(arc.length(Direction::Counterclockwise), 3.0 * PI);

        let arc = Arc::<SomeFrame> {
            circle: Circle {
                center: point![1.0, 1.0],
                radius: 2.0,
            },
            start: point![2.0, 1.0],
            end: point![1.0, 2.0],
        };
        assert_relative_eq!(arc.length(Direction::Clockwise), 3.0 * PI);
        assert_relative_eq!(arc.length(Direction::Counterclockwise), PI);
    }

    #[test]
    fn arc_cost_generic() {
        for angle_index in 0..100 {
            let angle = angle_index as f32 / 100.0 * TAU;
            for angle_distance_index in 1..100 {
                let angle_distance = angle_distance_index as f32 / 100.0 * TAU;
                let start = UnitComplex::<SomeFrame, SomeFrame>::new(angle) * vector![1.0, 0.0];
                let end = UnitComplex::<SomeFrame, SomeFrame>::new(angle + angle_distance)
                    * vector![1.0, 0.0];
                let center = point![PI, 4.20];
                let radius = 5.0;
                let arc = Arc::<SomeFrame> {
                    circle: Circle { center, radius },
                    start: center + start,
                    end: center + end,
                };

                println!("angle: {angle} angle_distance {angle_distance}");
                assert_relative_eq!(
                    arc.length(Direction::Counterclockwise),
                    radius * angle_distance,
                    epsilon = 0.001
                );
                assert_relative_eq!(
                    arc.length(Direction::Clockwise),
                    radius * (TAU - angle_distance),
                    epsilon = 0.001
                );
            }
        }
    }
}
