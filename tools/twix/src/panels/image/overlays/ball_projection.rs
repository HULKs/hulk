use std::{sync::Arc, time::Duration};

use color_eyre::Report;
use coordinate_systems::{Field, Ground, Pixel};
use eframe::egui::Color32;
use linear_algebra::{Isometry2, Point2};
use nalgebra::Matrix2;
use projection::{Projection, camera_matrix::CameraMatrix};
use ros_z::time::Time;
use ros_z_debug::SampleRecord;
use twix_visualization::twix_painter::TwixPainter;
use types::{field_dimensions::FieldDimensions, time_wrapper::TimeWrapper};

use super::super::image_overlay::OverlayObservation;
use crate::repaint::ObservationContext;

pub(super) const ALIGNMENT_TOLERANCE: Duration = Duration::from_millis(100);

pub(super) struct BallProjection {
    camera: OverlayObservation<TimeWrapper<CameraMatrix>>,
    dimensions: OverlayObservation<FieldDimensions>,
    ground_to_field: OverlayObservation<Isometry2<Ground, Field>>,
}

pub(super) struct BallFrame {
    pub camera: Arc<SampleRecord<TimeWrapper<CameraMatrix>>>,
    pub ball_radius: f32,
    ground_to_field: Option<Isometry2<Ground, Field>>,
}

impl BallProjection {
    pub fn new(context: &impl ObservationContext) -> Result<Self, Report> {
        Ok(Self {
            camera: OverlayObservation::new(context, "camera_matrix")?,
            dimensions: OverlayObservation::latched(context, "field_dimensions")?,
            ground_to_field: OverlayObservation::new(context, "ground_to_field")?,
        })
    }

    pub fn at_time(&self, time: Time) -> Option<BallFrame> {
        Some(BallFrame {
            camera: self.camera.nearest_to_time(time, ALIGNMENT_TOLERANCE)?,
            ball_radius: self.dimensions.latest()?.value.ball_radius,
            ground_to_field: self
                .ground_to_field
                .nearest_source_time(time, ALIGNMENT_TOLERANCE)
                .map(|sample| sample.value),
        })
    }

    pub fn ground_transform(
        &self,
        frame: &BallFrame,
        sample_time: Time,
    ) -> Isometry2<Ground, Ground> {
        match (
            frame.ground_to_field,
            self.ground_to_field
                .nearest_source_time(sample_time, ALIGNMENT_TOLERANCE),
        ) {
            (Some(current), Some(sample)) => current.inverse() * sample.value,
            _ => Isometry2::identity(),
        }
    }
}

impl BallFrame {
    pub fn paint_ground(
        &self,
        painter: &TwixPainter<Pixel>,
        position: Point2<Ground>,
        color: Color32,
    ) {
        let camera = &self.camera.value.inner;
        let Ok(pixel) = camera.ground_with_z_to_pixel(position, self.ball_radius) else {
            return;
        };
        let Ok(radius) = camera.get_pixel_radius(self.ball_radius, pixel) else {
            return;
        };
        if radius.is_finite() && radius > 0.0 {
            painter.ball(pixel, radius, color);
        }
    }

    pub fn paint_field(
        &self,
        painter: &TwixPainter<Pixel>,
        position: Point2<Field>,
        color: Color32,
    ) {
        if let Some(ground_to_field) = self.ground_to_field {
            self.paint_ground(painter, ground_to_field.inverse() * position, color);
        }
    }
}

// Linearize the camera projection at ball height: C_pixel = J C_ground Jᵀ.
pub(super) fn project_covariance(
    camera: &CameraMatrix,
    position: Point2<Ground>,
    height: f32,
    covariance: Matrix2<f32>,
) -> Option<(Point2<Pixel>, Matrix2<f32>)> {
    let pixel = camera.ground_with_z_to_pixel(position, height).ok()?;
    let projection = camera.intrinsics.as_matrix() * camera.ground_to_camera.inner.to_homogeneous();
    let homogeneous = projection * nalgebra::vector![position.x(), position.y(), height, 1.0];
    let jacobian = Matrix2::from_fn(|row, column| {
        (projection[(row, column)] - pixel.inner[row] * projection[(2, column)]) / homogeneous.z
    });
    let projected = jacobian * covariance * jacobian.transpose();
    (pixel.inner.coords.iter().all(|value| value.is_finite())
        && projected.iter().all(|value| value.is_finite()))
    .then_some((pixel, projected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::{Isometry3, point, vector};

    fn camera() -> CameraMatrix {
        CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![0.5, 0.5],
            nalgebra::point![0.5, 0.5],
            vector![640.0, 480.0],
            Isometry3::identity(),
            Isometry3::identity(),
            Isometry3::from_translation(0.0, 0.0, 1.0),
        )
    }

    #[test]
    fn covariance_uses_camera_scale_and_preserves_correlation() {
        let (pixel, covariance) = project_covariance(
            &camera(),
            point![0.0, 0.0],
            0.1,
            nalgebra::matrix![0.04, 0.01; 0.01, 0.09],
        )
        .unwrap();
        assert_eq!(pixel, point![320.0, 240.0]);
        let scale = Matrix2::from_diagonal(&nalgebra::vector![320.0 / 1.1, 240.0 / 1.1]);
        let expected = scale * nalgebra::matrix![0.04, 0.01; 0.01, 0.09] * scale;
        assert!((covariance - expected).norm() < 0.01);
    }

    #[test]
    fn rejects_balls_behind_camera() {
        assert!(
            project_covariance(&camera(), point![0.0, 0.0], -2.0, Matrix2::identity()).is_none()
        );
    }

    #[test]
    fn tilted_camera_covariance_matches_projected_displacements() {
        let mut camera = camera();
        camera.head_to_camera =
            camera.head_to_camera * Isometry3::from_rotation(vector![0.3, -0.4, 0.2]);
        camera.compute_memoized();
        let position = point![0.4, 0.2];
        let covariance = nalgebra::matrix![0.04, 0.01; 0.01, 0.09];
        let (_, projected) = project_covariance(&camera, position, 0.1, covariance).unwrap();
        let step = 0.001;
        let derivative = |offset| {
            let plus = camera
                .ground_with_z_to_pixel(position + offset, 0.1)
                .unwrap();
            let minus = camera
                .ground_with_z_to_pixel(position - offset, 0.1)
                .unwrap();
            (plus - minus).inner / (2.0 * step)
        };
        let numerical = Matrix2::from_columns(&[
            derivative(vector![step, 0.0]),
            derivative(vector![0.0, step]),
        ]);
        let expected = numerical * covariance * numerical.transpose();
        assert!((projected - expected).norm() / expected.norm() < 0.001);
    }
}
