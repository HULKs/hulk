pub mod arc;
pub mod circle;
pub mod circle_tangents;
pub mod convex_hull;
pub mod direction;
pub mod is_inside_polygon;
pub mod line;
pub mod line_segment;
pub mod look_at;
pub mod rectangle;
pub mod two_line_segments;

use std::f32::consts::{FRAC_PI_2, PI};

use linear_algebra::{Rotation2, Vector2};

pub trait Distance<T> {
    fn squared_distance_to(&self, other: T) -> f32;

    fn distance_to(&self, other: T) -> f32 {
        self.squared_distance_to(other).sqrt()
    }
}

fn signed_acute_angle<Frame>(first: Vector2<Frame>, second: Vector2<Frame>) -> f32 {
    let difference = Rotation2::rotation_between(first, second).angle();
    if difference > FRAC_PI_2 {
        difference - PI
    } else if difference < -FRAC_PI_2 {
        difference + PI
    } else {
        difference
    }
}
