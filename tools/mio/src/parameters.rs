use bevy::prelude::*;
use types::field_dimensions::FieldDimensions;

const SPL_FIELD_DIMENSIONS: FieldDimensions = FieldDimensions {
    ball_radius: 0.05,
    length: 7.5,
    width: 5.0,
    line_width: 0.05,
    penalty_marker_size: 0.1,
    goal_box_area_length: 0.6,
    goal_box_area_width: 2.2,
    penalty_area_length: 1.65,
    penalty_area_width: 3.7,
    penalty_marker_distance: 1.3,
    center_circle_diameter: 1.25,
    border_strip_width: 0.4,
    goal_inner_width: 1.5,
    goal_post_diameter: 0.1,
    goal_depth: 0.5,
};

#[derive(Resource)]
pub struct Parameters {
    pub field_dimensions: FieldDimensions,
}

impl Default for Parameters {
    fn default() -> Self {
        Parameters {
            field_dimensions: SPL_FIELD_DIMENSIONS,
        }
    }
}
