use coordinate_systems::{Ground, Pixel};
use linear_algebra::{distance_squared, point, vector, Point2, Vector2};
use projection::{
    camera_matrix::CameraMatrix, camera_projection::InverseCameraProjection,
    Error as ProjectionError, Projection,
};

use types::field_dimensions::FieldDimensions;

use crate::{
    center_circle::measurement::Measurement,
    corrections::{get_corrected_camera_matrix, Corrections},
    residuals::CalculateResiduals,
};

use super::fine_tuner::ellifit;

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
        let radius = field_dimensions.center_circle_diameter / 2.0;

        let min_y_point = measurement.circle_and_points.bounding_box.min;
        let max_y_point = measurement.circle_and_points.bounding_box.max;
        if !corrected
            .horizon
            .is_none_or(|horizon| horizon.is_above_with_margin(min_y_point, 5.0))
        {
            return Err(ProjectionError::NotOnProjectionPlane);
        };

        let pixel_y_range = max_y_point.y() - min_y_point.y();
        let pixel_to_ground = &corrected.pixel_to_ground;

        // we are skipping variance along x axis[pixel] for now.
        // min weight = 1.0
        let max_weight =
            max_weight_calculation_unchecked(&min_y_point, &max_y_point, pixel_to_ground).y();
        let min_y = min_y_point.y();

        let residuals = CenterCircleResiduals {
            radial_residuals: measurement
                .circle_and_points
                .points
                .iter()
                .map(|&point| {
                    let projected = pixel_to_ground.back_project_unchecked(point).xy();
                    let residual = average_circle_residual(projected, projected_center, radius);
                    let weight = interpolate(point.y(), min_y, pixel_y_range, max_weight);
                    residual * weight
                })
                .collect(),
        };

        Ok(residuals)
    }
}

#[inline(always)]
fn average_circle_residual<'a>(
    projected_point: Point2<Ground>,
    center: Point2<Ground>,
    radius: f32,
) -> f32 {
    let center_to_point_distance = (projected_point - center).norm();
    center_to_point_distance - radius
}

/// Interpolate weight based on the y coordinate of pixel.
#[inline(always)]
fn interpolate(pixel_y: f32, pixel_y_min: f32, pixel_y_range: f32, weight_max: f32) -> f32 {
    1.0 + (weight_max / pixel_y_range) * (pixel_y - pixel_y_min)
}

/// Calculates minimum and maximum variations (kinda like covariance) at top and bottom of circle.
/// This kind of calculation is needed as the pixel noise (+-0.5) has different variances in the ground plane after projection.
/// Therefore it has to be compensated in the residual calculation to avoid biasing towards further away points
fn max_weight_calculation_unchecked(
    min_y_point: &Point2<Pixel>,
    max_y_point: &Point2<Pixel>,
    pixel_to_ground: &InverseCameraProjection<Ground>,
) -> Vector2<Ground> {
    // Note: This is an approximation, a more precise value could/ should be calculated.
    // highest y (closest to nao) has lowest variance as the camera looks down.

    let get_variance = |point: &Point2<Pixel>| {
        let inner = &point.coords().inner;
        pixel_to_ground
            .back_project_unchecked(inner.add_scalar(0.5).into())
            .xy()
            - pixel_to_ground
                .back_project_unchecked(inner.add_scalar(-0.5).into())
                .xy()
    };

    // max_y = closest to robot
    let variance_min = get_variance(max_y_point);
    let variance_max = get_variance(min_y_point);

    // rescale
    // variance_max /= variance_min;
    // variance_min /= variance_min; // one!

    // as weight
    // 1.0/variance

    // combined scaling and weight (reciprocal)
    // variance_min / variance_max
    vector![
        variance_min.x() / variance_max.x(),
        variance_min.y() / variance_max.y()
    ]
}

impl From<CenterCircleResiduals> for Vec<f32> {
    fn from(residuals: CenterCircleResiduals) -> Self {
        residuals.radial_residuals
    }
}

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn test() {

//         let radial_direction_in_ground = corrected.pixel_to_ground(*point).ok()?.coords();
//         radial_direction_in_ground.normalize();

//         let circle_point =
//             (inner_radius * radial_direction_in_ground.inner + projected_center.coords().inner);
//     }
// }
