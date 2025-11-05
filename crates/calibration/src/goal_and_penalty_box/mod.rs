mod lines;

pub use lines::{LineType, Measurement, Residuals};

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use coordinate_systems::Pixel;
    use geometry::line_segment::LineSegment;
    use levenberg_marquardt::LevenbergMarquardt;
    use linear_algebra::{point, vector, Isometry2, Isometry3, Vector2};
    use projection::camera_matrix::CameraMatrix;
    use types::field_dimensions::FieldDimensions;

    use crate::{corrections::Corrections, problem::CalibrationProblem};

    use super::{LineType, Measurement, Residuals};

    fn from_normalized_focal_and_center_short(
        focal_length: nalgebra::Vector2<f32>,
        optical_center: nalgebra::Point2<f32>,
        image_size: Vector2<Pixel>,
    ) -> CameraMatrix {
        CameraMatrix::from_normalized_focal_and_center(
            focal_length,
            optical_center,
            image_size,
            Isometry3::identity(),
            Isometry3::identity(),
            Isometry3::from_translation(0.0, 0.0, 1.0),
        )
    }

    #[test]
    pub fn does_not_optimize_when_optimal() {
        let camera_matrix = from_normalized_focal_and_center_short(
            nalgebra::vector![1.0, 1.0],
            nalgebra::point![0.5, 0.5],
            vector![640.0, 480.0],
        );
        let corrections = Corrections::default();

        let field_to_ground = Isometry2::identity();

        let measurement = Measurement {
            line_type: LineType::Goal,
            line_segment: LineSegment::new(point![3200.0, 1680.0], point![3200.0, -1200.0]),
            camera_matrix,
            field_to_ground,
        };

        let field_dimensions = FieldDimensions::SPL_2025;
        let problem =
            CalibrationProblem::<Residuals>::new(corrections, vec![measurement], field_dimensions);
        let (result, _report) = LevenbergMarquardt::new().minimize(problem);
        assert_relative_eq!(result.get_corrections(), corrections);
    }
}
