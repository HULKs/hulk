use std::f32::consts::FRAC_PI_2;

use nalgebra::{DVector, Dyn, Owned, Vector};
use types::FieldDimensions;

use crate::{
    corrections::Corrections,
    lines::{LinesError, LinesPerCamera},
    measurement::Measurement,
};

pub type Residual = Vector<f32, Dyn, ResidualStorage>;
pub type ResidualStorage = Owned<f32, Dyn>;

pub fn calculate_residuals_from_parameters(
    parameters: &Corrections,
    measurements: &[Measurement],
    field_dimensions: &FieldDimensions,
) -> Option<Residual> {
    measurements
        .iter()
        .fold(
            Some(Vec::new()),
            |residuals: Option<Vec<f32>>, measurements| match residuals {
                Some(mut residuals) => {
                    residuals.extend::<Vec<f32>>(
                        Residuals::calculate_from(parameters, measurements, field_dimensions)
                            .ok()?
                            .into(),
                    );
                    Some(residuals)
                }
                None => None,
            },
        )
        .map(DVector::from_vec)
}

pub struct Residuals {
    pub top: ResidualsPerCamera,
    pub bottom: ResidualsPerCamera,
}

impl Residuals {
    pub fn calculate_from(
        parameters: &Corrections,
        measurement: &Measurement,
        field_dimensions: &FieldDimensions,
    ) -> Result<Self, ResidualsError> {
        let corrected = measurement.matrices.to_corrected(
            parameters.correction_in_robot,
            parameters.correction_in_camera_top,
            parameters.correction_in_camera_bottom,
        );

        let projected_lines = measurement
            .lines
            .to_projected(&corrected)
            .map_err(ResidualsError::NotProjected)?;

        Ok(Self {
            top: ResidualsPerCamera::from_lines_and_field_dimensions(
                &projected_lines.top,
                field_dimensions,
            ),
            bottom: ResidualsPerCamera::from_lines_and_field_dimensions(
                &projected_lines.bottom,
                field_dimensions,
            ),
        })
    }
}

impl From<Residuals> for Vec<f32> {
    fn from(residuals: Residuals) -> Self {
        vec![
            residuals.top.border_to_connecting_angle,
            residuals.top.connecting_to_goal_box_angle,
            residuals.top.distance_between_parallel_line_start_points,
            residuals.top.distance_between_parallel_line_center_points,
            residuals.top.distance_between_parallel_line_end_points,
        ]
    }
}

pub struct ResidualsPerCamera {
    pub border_to_connecting_angle: f32,
    pub connecting_to_goal_box_angle: f32,
    pub distance_between_parallel_line_start_points: f32,
    pub distance_between_parallel_line_center_points: f32,
    pub distance_between_parallel_line_end_points: f32,
}

impl ResidualsPerCamera {
    fn from_lines_and_field_dimensions(
        lines: &LinesPerCamera,
        field_dimensions: &FieldDimensions,
    ) -> Self {
        let border_to_connecting_angle = lines.border_line.angle(lines.connecting_line);
        let connecting_to_goal_box_angle = lines.border_line.angle(lines.connecting_line);
        let distance_between_parallel_line_start_points =
            lines.border_line.distance_to_point(lines.goal_box_line.0);
        let distance_between_parallel_line_center_points = lines
            .border_line
            .distance_to_point(lines.goal_box_line.center());
        let distance_between_parallel_line_end_points =
            lines.border_line.distance_to_point(lines.goal_box_line.1);

        ResidualsPerCamera {
            border_to_connecting_angle: border_to_connecting_angle - FRAC_PI_2,
            connecting_to_goal_box_angle: connecting_to_goal_box_angle - FRAC_PI_2,
            distance_between_parallel_line_start_points: distance_between_parallel_line_start_points
                - field_dimensions.goal_box_area_length,
            distance_between_parallel_line_center_points:
                distance_between_parallel_line_center_points - field_dimensions.goal_box_area_length,
            distance_between_parallel_line_end_points: distance_between_parallel_line_end_points
                - field_dimensions.goal_box_area_length,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ResidualsError {
    #[error("failed to project measurements to ground")]
    NotProjected(#[source] LinesError),
}
