use color_eyre::Result;
use coordinate_systems::Ground;
use linear_algebra::Point2;
use nalgebra::{Dyn, Owned, Vector};
use projection::Error;
use projection::Projection;
use types::field_dimensions::FieldDimensions;

use crate::{
    center_circle::measurement::Measurement,
    corrections::{get_corrected_camera_matrix, Corrections},
    residuals::CalculateResiduals,
};

pub type Residual = Vector<f32, Dyn, ResidualStorage>;
pub type ResidualStorage = Owned<f32, Dyn>;

// TODO move above common parts
pub struct Residuals {
    residual_values: Vec<f32>,
}

impl CalculateResiduals<Measurement> for Residuals {
    fn calculate_from(
        parameters: &Corrections,
        measurement: &Measurement,
        field_dimensions: &FieldDimensions,
    ) -> Result<Self> {
        let corrected =
            get_corrected_camera_matrix(&measurement.matrix, measurement.position, parameters);

        let radius_squared = field_dimensions.center_circle_diameter / 2.0;

        let projected_center = corrected.pixel_to_ground(measurement.circles.center)?;
        let projected_points: Vec<Point2<Ground>> = measurement
            .circles
            .points
            .iter()
            .filter_map(|&point| corrected.pixel_to_ground(point).ok())
            .collect();

        // TODO figure out a better way
        let has_projection_error = projected_points.len() != measurement.circles.points.len();
        if has_projection_error {
            return Err(Error::NotOnProjectionPlane.into());
        }
        let residual_values = projected_points
            .into_iter()
            .map(|projected_point| {
                (projected_point - projected_center).norm_squared() - radius_squared
            })
            .collect();

        Ok(Residuals { residual_values })
    }
}

impl From<Residuals> for Vec<f32> {
    fn from(residuals: Residuals) -> Self {
        residuals.residual_values
    }
}
