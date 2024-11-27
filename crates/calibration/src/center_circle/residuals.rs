use coordinate_systems::Ground;
use linear_algebra::Point2;
use projection::Error as ProjectionError;
use projection::Projection;
use types::field_dimensions::FieldDimensions;

use crate::{
    center_circle::measurement::Measurement,
    corrections::{get_corrected_camera_matrix, Corrections},
    residuals::CalculateResiduals,
};

pub struct CenterCircleResiduals {
    radial_residuals: Vec<f32>,
}

impl CalculateResiduals for CenterCircleResiduals {
    type Error = ProjectionError;
    type Measurement = Measurement;

    fn calculate_from(
        parameters: &Corrections,
        measurement: &Measurement,
        field_dimensions: &FieldDimensions,
    ) -> Result<Self, Self::Error> {
        let corrected =
            get_corrected_camera_matrix(&measurement.matrix, measurement.position, parameters);

        let projected_center = corrected.pixel_to_ground(measurement.circle_and_points.center)?;
        let projected_points: Vec<Point2<Ground>> = measurement
            .circle_and_points
            .points
            .iter()
            .filter_map(|&point| corrected.pixel_to_ground(point).ok())
            .collect();

        if projected_points.len() != measurement.circle_and_points.points.len() {
            return Err(ProjectionError::NotOnProjectionPlane);
        }

        let radius = field_dimensions.center_circle_diameter / 2.0;
        let line_width_half = field_dimensions.line_width / 2.0;
        let inner_radius = radius - line_width_half;
        let outer_radius = radius + line_width_half;

        let radial_residuals = projected_points
            .into_iter()
            .map(|projected_point| {
                let center_to_point_distance = (projected_point - projected_center).norm();
                let inner_error = center_to_point_distance - inner_radius;
                let outer_error = outer_radius - center_to_point_distance;
                if inner_error.abs() < outer_error.abs() {
                    inner_error
                } else {
                    outer_error
                }
            })
            .collect();

        Ok(CenterCircleResiduals { radial_residuals })
    }
}

impl From<CenterCircleResiduals> for Vec<f32> {
    fn from(residuals: CenterCircleResiduals) -> Self {
        residuals.radial_residuals
    }
}
