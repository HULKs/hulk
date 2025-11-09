use coordinate_systems::{Field, Ground, Pixel};
use geometry::line_segment::LineSegment;
use linear_algebra::{distance, Isometry2};
use projection::{camera_matrix::CameraMatrix, Error as ProjectionError, Projection};
use types::field_dimensions::{FieldDimensions, Half, Side};

use crate::{
    corrections::{get_corrected_camera_matrix, Corrections},
    residuals::CalculateResiduals,
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum LineType {
    Goal,
    FrontGoalArea,
    LeftGoalArea,
    RightGoalArea,
    FrontPenaltyArea,
    LeftPenaltyArea,
    RightPenaltyArea,
}

impl LineType {
    pub fn line_segment(
        self,
        field_dimensions: &FieldDimensions,
        half: Half,
    ) -> LineSegment<Field> {
        let (first, second) = match self {
            Self::Goal => (
                field_dimensions.corner(half, Side::Left),
                field_dimensions.corner(half, Side::Right),
            ),
            Self::FrontGoalArea => (
                field_dimensions.goal_box_corner(half, Side::Left),
                field_dimensions.goal_box_corner(half, Side::Right),
            ),
            Self::LeftGoalArea => (
                field_dimensions.goal_box_corner(half, Side::Left),
                field_dimensions.goal_box_goal_line_intersection(half, Side::Left),
            ),
            Self::RightGoalArea => (
                field_dimensions.goal_box_corner(half, Side::Right),
                field_dimensions.goal_box_goal_line_intersection(half, Side::Right),
            ),
            Self::FrontPenaltyArea => (
                field_dimensions.penalty_box_corner(half, Side::Left),
                field_dimensions.penalty_box_corner(half, Side::Right),
            ),
            Self::LeftPenaltyArea => (
                field_dimensions.penalty_box_corner(half, Side::Left),
                field_dimensions.penalty_box_goal_line_intersection(half, Side::Left),
            ),
            Self::RightPenaltyArea => (
                field_dimensions.penalty_box_corner(half, Side::Right),
                field_dimensions.penalty_box_goal_line_intersection(half, Side::Right),
            ),
        };
        LineSegment::new(first, second)
    }
}

pub struct Measurement<Frame> {
    pub line_type: LineType,
    pub line_segment: LineSegment<Frame>,
    pub camera_matrix: CameraMatrix,
    pub field_to_ground: Isometry2<Field, Ground>,
}

pub fn line_segment_error(expected: LineSegment<Pixel>, drawn: LineSegment<Pixel>) -> (f32, f32) {
    let t_0 = expected.projection_factor(drawn.0);
    let intersection_0 = expected.0 + (expected.1 - expected.0) * t_0;

    let t_1 = expected.projection_factor(drawn.1);
    let intersection_1 = expected.0 + (expected.1 - expected.0) * t_1;

    (
        distance(drawn.0, intersection_0),
        distance(drawn.1, intersection_1),
    )
}

#[derive(Debug)]
pub struct Residuals {
    left_projected_residual: f32,
    right_projected_residual: f32,
}

impl From<Residuals> for Vec<f32> {
    fn from(residuals: Residuals) -> Self {
        vec![
            residuals.left_projected_residual,
            residuals.right_projected_residual,
        ]
    }
}

impl CalculateResiduals for Residuals {
    type Error = ProjectionError;
    type Measurement = Measurement<Pixel>;

    fn calculate_from(
        parameters: &Corrections,
        measurement: &Self::Measurement,
        field_dimensions: &FieldDimensions,
    ) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let camera_matrix = get_corrected_camera_matrix(&measurement.camera_matrix, parameters);

        let expected_line = measurement
            .line_type
            .line_segment(field_dimensions, Half::Opponent);
        let expected_line = expected_line
            .try_map(|point| camera_matrix.ground_to_pixel(measurement.field_to_ground * point))?;
        let (left_projected_residual, right_projected_residual) =
            line_segment_error(expected_line, measurement.line_segment);

        Ok(Residuals {
            left_projected_residual,
            right_projected_residual,
        })
    }
}
