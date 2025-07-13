use approx::{AbsDiffEq, RelativeEq};
use serde::{Deserialize, Serialize};

use linear_algebra::{Orientation2, Point2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::{
    circle::Circle,
    direction::{AngleTo, Direction},
};

#[derive(
    Clone, Copy, Debug, Deserialize, PathDeserialize, PathIntrospect, PathSerialize, Serialize,
)]
pub struct Arc<Frame> {
    pub circle: Circle<Frame>,
    pub start: Orientation2<Frame>,
    pub end: Orientation2<Frame>,
    pub direction: Direction,
}

impl<Frame> Arc<Frame> {
    pub fn new(
        circle: Circle<Frame>,
        start: Orientation2<Frame>,
        end: Orientation2<Frame>,
        direction: Direction,
    ) -> Self {
        Self {
            circle,
            start,
            end,
            direction,
        }
    }

    pub fn length(&self) -> f32 {
        let angle = self.start.angle_to(self.end, self.direction);

        angle * self.circle.radius
    }

    pub fn start_point(&self) -> Point2<Frame> {
        self.circle.point_at_angle(self.start)
    }

    pub fn end_point(&self) -> Point2<Frame> {
        self.circle.point_at_angle(self.end)
    }
}

impl<Frame> PartialEq for Arc<Frame> {
    fn eq(&self, other: &Self) -> bool {
        self.circle == other.circle
            && self.start == other.start
            && self.end == other.end
            && self.direction == other.direction
    }
}

impl<Frame> AbsDiffEq for Arc<Frame> {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        Self::Epsilon::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.direction == other.direction
            && self.circle.abs_diff_eq(&other.circle, epsilon)
            && self.start.abs_diff_eq(&other.start, epsilon)
            && self.end.abs_diff_eq(&other.end, epsilon)
    }
}

impl<Frame> RelativeEq for Arc<Frame> {
    fn default_max_relative() -> f32 {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.direction == other.direction
            && self
                .circle
                .relative_eq(&other.circle, epsilon, max_relative)
            && self.start.relative_eq(&other.start, epsilon, max_relative)
            && self.end.relative_eq(&other.end, epsilon, max_relative)
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, PI, TAU};

    use approx::assert_relative_eq;

    use linear_algebra::point;

    use super::*;

    #[derive(Clone, Copy)]
    struct SomeFrame;

    #[test]
    fn arc_cost_90_degrees() {
        let arc = Arc::<SomeFrame> {
            circle: Circle {
                center: point![1.0, 1.0],
                radius: 2.0,
            },
            start: Orientation2::new(FRAC_PI_2),
            end: Orientation2::new(0.0),
            direction: Direction::Clockwise,
        };
        assert_relative_eq!(arc.length(), PI);

        let arc = Arc::<SomeFrame> {
            circle: Circle {
                center: point![1.0, 1.0],
                radius: 2.0,
            },
            start: Orientation2::new(FRAC_PI_2),
            end: Orientation2::new(0.0),
            direction: Direction::Counterclockwise,
        };
        assert_relative_eq!(arc.length(), 3.0 * PI);

        let arc = Arc::<SomeFrame> {
            circle: Circle {
                center: point![1.0, 1.0],
                radius: 2.0,
            },
            start: Orientation2::new(0.0),
            end: Orientation2::new(FRAC_PI_2),
            direction: Direction::Clockwise,
        };
        assert_relative_eq!(arc.length(), 3.0 * PI);

        let arc = Arc::<SomeFrame> {
            circle: Circle {
                center: point![1.0, 1.0],
                radius: 2.0,
            },
            start: Orientation2::new(0.0),
            end: Orientation2::new(FRAC_PI_2),
            direction: Direction::Counterclockwise,
        };
        assert_relative_eq!(arc.length(), PI);
    }

    #[test]
    fn arc_cost_generic() {
        for angle_index in 0..100 {
            let angle = angle_index as f32 / 100.0 * TAU;
            for angle_distance_index in 1..100 {
                let angle_distance = angle_distance_index as f32 / 100.0 * TAU;
                let center = point![PI, 4.20];
                let radius = 5.0;

                println!("angle: {angle} angle_distance {angle_distance}");

                let arc = Arc::<SomeFrame> {
                    circle: Circle { center, radius },
                    start: Orientation2::new(angle),
                    end: Orientation2::new(angle + angle_distance),
                    direction: Direction::Counterclockwise,
                };
                assert_relative_eq!(arc.length(), radius * angle_distance, epsilon = 0.001);

                let arc = Arc::<SomeFrame> {
                    circle: Circle { center, radius },
                    start: Orientation2::new(angle),
                    end: Orientation2::new(angle + angle_distance),
                    direction: Direction::Clockwise,
                };
                assert_relative_eq!(
                    arc.length(),
                    radius * (TAU - angle_distance),
                    epsilon = 0.001
                );
            }
        }
    }
}
