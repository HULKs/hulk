pub mod angle;
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

pub trait Distance<T> {
    fn squared_distance_to(&self, other: T) -> f32;

    fn distance_to(&self, other: T) -> f32 {
        self.squared_distance_to(other).sqrt()
    }
}
