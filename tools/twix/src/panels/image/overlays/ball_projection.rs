use std::{sync::Arc, time::Duration};

use color_eyre::Report;
use coordinate_systems::{Field, Ground, Pixel};
use eframe::egui::Color32;
use linear_algebra::{Isometry2, Point2};
use nalgebra::Matrix2;
use projection::{Projection, camera_matrix::CameraMatrix};
use ros_z::{
    qos::{QosDurability, QosProfile},
    time::Time,
};
use ros_z_debug::{ObservationPolicy, SampleRecord, query::plan::Selection};
use twix_visualization::twix_painter::TwixPainter;
use types::{field_dimensions::FieldDimensions, time_wrapper::TimeWrapper};

use super::{
    super::inputs::{OverlayInput, OverlayInputs},
    camera::{interpolate_transform, interpolation_endpoints},
};

pub(super) const ALIGNMENT_TOLERANCE: Duration = Duration::from_millis(100);

#[derive(Default)]
pub(super) struct BallProjection {
    frame: Option<BallFrame>,
    ground_to_field: Vec<Arc<SampleRecord<Isometry2<Ground, Field>>>>,
}

pub(super) struct BallFrame {
    pub camera: CameraMatrix,
    pub ball_radius: f32,
    ground_to_field: Option<Isometry2<Ground, Field>>,
}

impl BallProjection {
    pub fn inputs() -> Vec<OverlayInput> {
        let mut dimensions =
            OverlayInput::source_time::<FieldDimensions>("field_dimensions", Selection::latest());
        dimensions.source.live_policy =
            ObservationPolicy::default().with_subscriber_qos(QosProfile {
                durability: QosDurability::TransientLocal,
                ..Default::default()
            });
        vec![
            OverlayInput::new::<TimeWrapper<CameraMatrix>>(
                "camera_matrix",
                Selection::Bracket {
                    max_gap: ALIGNMENT_TOLERANCE * 2,
                },
            ),
            dimensions,
            OverlayInput::source_time::<Isometry2<Ground, Field>>(
                "ground_to_field",
                Selection::Window {
                    before: ALIGNMENT_TOLERANCE * 2,
                    after: ALIGNMENT_TOLERANCE * 2,
                },
            ),
        ]
    }

    pub fn prepare(
        &mut self,
        time: Time,
        inputs: &mut OverlayInputs,
        offset: usize,
    ) -> Result<(), Report> {
        self.frame = None;
        self.ground_to_field.clear();
        let Some(camera) = inputs.decode::<TimeWrapper<CameraMatrix>>(offset, time)? else {
            return Ok(());
        };
        let Some(dimensions) = inputs.decode::<FieldDimensions>(offset + 1, time)? else {
            return Ok(());
        };
        // Frame transforms need the image time and the filter snapshot time.
        self.ground_to_field = inputs.records(offset + 2).unwrap_or_default();
        let (a, b, fraction) = interpolation_endpoints(&camera, time);
        let (a, b) = (&a.inner, &b.inner);
        if a.intrinsics != b.intrinsics || a.image_size != b.image_size {
            return Ok(());
        }
        let mut camera = a.clone();
        camera.ground_to_robot =
            interpolate_transform(a.ground_to_robot, b.ground_to_robot, fraction);
        camera.robot_to_head = interpolate_transform(a.robot_to_head, b.robot_to_head, fraction);
        camera.head_to_camera = interpolate_transform(a.head_to_camera, b.head_to_camera, fraction);
        camera.compute_memoized();
        self.frame = Some(BallFrame {
            camera,
            ball_radius: dimensions.last().value.ball_radius,
            ground_to_field: nearest(&self.ground_to_field, time).map(|sample| sample.value),
        });
        Ok(())
    }

    pub fn frame(&self) -> Option<&BallFrame> {
        self.frame.as_ref()
    }

    pub fn ground_transform(
        &self,
        frame: &BallFrame,
        sample_time: Time,
    ) -> Isometry2<Ground, Ground> {
        match (
            frame.ground_to_field,
            nearest(&self.ground_to_field, sample_time),
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
        let camera = &self.camera;
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

pub(super) fn aligned_window() -> Selection {
    Selection::Window {
        before: ALIGNMENT_TOLERANCE,
        after: ALIGNMENT_TOLERANCE,
    }
}

pub(super) fn nearest<T>(
    records: &[Arc<SampleRecord<T>>],
    time: Time,
) -> Option<&Arc<SampleRecord<T>>> {
    records
        .iter()
        .rev()
        .min_by_key(|record| record.effective_time.as_nanos().abs_diff(time.as_nanos()))
        .filter(|record| {
            record.effective_time.as_nanos().abs_diff(time.as_nanos())
                <= ALIGNMENT_TOLERANCE.as_nanos() as u64
        })
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
    fn projection_interpolates_camera_and_transforms_filter_snapshot_coordinates() {
        use crate::panels::image::tests::raw_sample;
        use ros_z_debug::query::plan::State;

        let time = Time::from_nanos(1_000_000_000);
        let camera_record = |nanos, x| {
            let mut camera = camera();
            camera.ground_to_robot = Isometry3::from_translation(x, 0.0, 0.0);
            raw_sample(
                "/camera",
                nanos as u64,
                nanos,
                nanos,
                &TimeWrapper {
                    time: Time::from_nanos(nanos),
                    inner: camera,
                },
            )
        };
        let mut inputs = OverlayInputs::default();
        inputs.update(&[
            State::Ready(vec![
                camera_record(990_000_000, 2.0),
                camera_record(1_010_000_000, 4.0),
            ]),
            State::Ready(vec![raw_sample(
                "/dimensions",
                1,
                1,
                1,
                &FieldDimensions::SPL_2025,
            )]),
            State::Ready(vec![
                raw_sample(
                    "/ground",
                    2,
                    900_000_000,
                    900_000_000,
                    &Isometry2::<Ground, Field>::from_parts(vector![0.5, 0.0], 0.0),
                ),
                raw_sample(
                    "/ground",
                    3,
                    1_000_000_000,
                    1_000_000_000,
                    &Isometry2::<Ground, Field>::from_parts(vector![1.0, 0.0], 0.0),
                ),
            ]),
        ]);
        let mut projection = BallProjection::default();
        projection.prepare(time, &mut inputs, 0).unwrap();
        let frame = projection.frame().unwrap();
        assert_eq!(frame.camera.ground_to_robot.inner.translation.x, 3.0);
        assert_eq!(
            projection.ground_transform(frame, Time::from_nanos(900_000_000)) * point![0.0, 0.0],
            point![-0.5, 0.0],
        );
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
