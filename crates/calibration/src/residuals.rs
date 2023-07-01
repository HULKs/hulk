use nalgebra::{DVector, Dyn, Owned, Vector};
use types::{CameraPosition, FieldDimensions};

use crate::{corrections::Corrections, lines::LinesError, measurement::Measurement};

pub type Residual = Vector<f32, Dyn, ResidualStorage>;
pub type ResidualStorage = Owned<f32, Dyn>;

pub fn calculate_residuals_from_parameters(
    parameters: &Corrections,
    measurements: &[Measurement],
    field_dimensions: &FieldDimensions,
) -> Option<Residual> {
    let mut residuals = Vec::new();
    for measurement in measurements {
        let residuals_part: Vec<f32> =
            Residuals::calculate_from(parameters, measurement, field_dimensions)
                .ok()?
                .into();
        residuals.extend(residuals_part);
    }

    Some(DVector::from_vec(residuals))
}

pub struct Residuals {
    pub border_to_connecting_angle: f32,
    pub connecting_to_goal_box_angle: f32,
    pub distance_between_parallel_line_start_points: f32,
    pub distance_between_parallel_line_center_points: f32,
    pub distance_between_parallel_line_end_points: f32,
}

impl Residuals {
    pub fn calculate_from(
        parameters: &Corrections,
        measurement: &Measurement,
        field_dimensions: &FieldDimensions,
    ) -> Result<Self, ResidualsError> {
        let corrected = measurement.matrix.to_corrected(
            parameters.correction_in_robot,
            match measurement.position {
                CameraPosition::Top => parameters.correction_in_camera_top,
                CameraPosition::Bottom => parameters.correction_in_camera_bottom,
            },
        );

        let projected_lines = measurement
            .lines
            .to_projected(&corrected)
            .map_err(ResidualsError::NotProjected)?;

        let border_to_connecting_angle = projected_lines
            .border_line
            .signed_acute_angle_to_orthogonal(projected_lines.connecting_line);
        let connecting_to_goal_box_angle = projected_lines
            .border_line
            .signed_acute_angle_to_orthogonal(projected_lines.connecting_line);
        let distance_between_parallel_line_start_points = projected_lines
            .border_line
            .distance_to_point(projected_lines.goal_box_line.0);
        let distance_between_parallel_line_center_points = projected_lines
            .border_line
            .distance_to_point(projected_lines.goal_box_line.center());
        let distance_between_parallel_line_end_points = projected_lines
            .border_line
            .distance_to_point(projected_lines.goal_box_line.1);

        Ok(Residuals {
            border_to_connecting_angle,
            connecting_to_goal_box_angle,
            distance_between_parallel_line_start_points: distance_between_parallel_line_start_points
                - field_dimensions.goal_box_area_length,
            distance_between_parallel_line_center_points:
                distance_between_parallel_line_center_points - field_dimensions.goal_box_area_length,
            distance_between_parallel_line_end_points: distance_between_parallel_line_end_points
                - field_dimensions.goal_box_area_length,
        })
    }
}

impl From<Residuals> for Vec<f32> {
    fn from(residuals: Residuals) -> Self {
        vec![
            residuals.border_to_connecting_angle,
            residuals.connecting_to_goal_box_angle,
            residuals.distance_between_parallel_line_start_points,
            residuals.distance_between_parallel_line_center_points,
            residuals.distance_between_parallel_line_end_points,
        ]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ResidualsError {
    #[error("failed to project measurements to ground")]
    NotProjected(#[source] LinesError),
}
