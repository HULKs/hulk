use serde::{Deserialize, Serialize};

use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use coordinate_systems::Field;

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct FieldDimensions {
    pub ball_radius: f32,
    pub length: f32,
    pub width: f32,
    pub line_width: f32,
    pub penalty_marker_size: f32,
    pub goal_box_area_length: f32,
    pub goal_box_area_width: f32,
    pub penalty_area_length: f32,
    pub penalty_area_width: f32,
    pub penalty_marker_distance: f32,
    pub center_circle_diameter: f32,
    pub border_strip_width: f32,
    pub goal_inner_width: f32,
    pub goal_post_diameter: f32,
    pub goal_depth: f32,
}

impl FieldDimensions {
    pub fn is_inside_field(&self, position: Point2<Field>) -> bool {
        position.x().abs() < self.length / 2.0 && position.y().abs() < self.width / 2.0
    }

    pub fn is_inside_any_goal_box(&self, position: Point2<Field>) -> bool {
        position.x().abs() > self.length / 2.0 - self.goal_box_area_length
            && position.y().abs() < self.goal_box_area_width / 2.0
    }

    pub fn is_inside_kick_off_target_region(&self, position: Point2<Field>) -> bool {
        position.x().signum() == position.y().signum()
            && position.x().abs() < self.width / 2.0
            && position.x().abs() <= position.y().abs()
    }
}
