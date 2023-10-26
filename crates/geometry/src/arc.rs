use std::f32::consts::PI;

use approx::{AbsDiffEq, RelativeEq};
use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{circle::Circle, orientation::Orientation};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct Arc {
    pub circle: Circle,
    pub start: Point2<f32>,
    pub end: Point2<f32>,
}

impl AbsDiffEq for Arc {
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

impl RelativeEq for Arc {
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

impl Arc {
    pub fn new(circle: Circle, start: Point2<f32>, end: Point2<f32>) -> Self {
        Self { circle, start, end }
    }

    pub fn length(&self, orientation: Orientation) -> f32 {
        let vector_start = self.start - self.circle.center;
        let vector_end = self.end - self.circle.center;

        let angle_x_axis_to_start = vector_start.y.atan2(vector_start.x);
        let mut angle = vector_end.y.atan2(vector_end.x) - angle_x_axis_to_start;

        if (orientation == Orientation::Clockwise) && (angle > 0.0) {
            angle -= 2.0 * PI;
            angle *= -1.0;
        }
        if (orientation == Orientation::Counterclockwise) && (angle < 0.0) {
            angle += 2.0 * PI;
        }
        (angle * self.circle.radius).abs()
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use approx::assert_relative_eq;
    use nalgebra::{point, vector, UnitComplex};

    use super::*;

    #[test]
    fn arc_cost_90_degrees() {
        let arc = Arc {
            circle: Circle {
                center: point![1.0, 1.0],
                radius: 2.0,
            },
            start: point![1.0, 2.0],
            end: point![2.0, 1.0],
        };
        assert_relative_eq!(arc.length(Orientation::Clockwise), PI);
        assert_relative_eq!(arc.length(Orientation::Counterclockwise), 3.0 * PI);

        let arc = Arc {
            circle: Circle {
                center: point![1.0, 1.0],
                radius: 2.0,
            },
            start: point![2.0, 1.0],
            end: point![1.0, 2.0],
        };
        assert_relative_eq!(arc.length(Orientation::Clockwise), 3.0 * PI);
        assert_relative_eq!(arc.length(Orientation::Counterclockwise), PI);
    }

    #[test]
    fn arc_cost_generic() {
        for angle_index in 0..100 {
            let angle = angle_index as f32 / 100.0 * 2.0 * PI;
            for angle_distance_index in 1..100 {
                let angle_distance = angle_distance_index as f32 / 100.0 * 2.0 * PI;
                let start = UnitComplex::from_angle(angle) * vector![1.0, 0.0];
                let end = UnitComplex::from_angle(angle + angle_distance) * vector![1.0, 0.0];
                let center = point![PI, 4.20];
                let radius = 5.0;
                let arc = Arc {
                    circle: Circle { center, radius },
                    start: center + start,
                    end: center + end,
                };

                println!("angle: {angle} angle_distance {angle_distance}");
                assert_relative_eq!(
                    arc.length(Orientation::Counterclockwise),
                    radius * angle_distance,
                    epsilon = 0.001
                );
                assert_relative_eq!(
                    arc.length(Orientation::Clockwise),
                    radius * (2.0 * PI - angle_distance),
                    epsilon = 0.001
                );
            }
        }
    }
}
