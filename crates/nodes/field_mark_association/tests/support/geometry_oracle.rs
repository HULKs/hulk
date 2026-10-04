use linear_algebra::{Isometry3, point};
use nalgebra::{Translation3, UnitComplex, UnitQuaternion, Vector2, Vector3};
use types::localization::PoseEstimate;

use super::{AssociationFixture, AssociationGeometry, Time, robot_to_camera, robot_to_local};

pub struct FitMetrics {
    pub camera_height: f64,
    pub free_height_scale: f64,
    pub metric_rms: f64,
    pub pixel_rms: f64,
}

/// Test-only prior from the recorded labels, not a production startup pose estimator.
/// Fit translation/yaw with measured height and tilt fixed; scale is diagnostic only.
pub fn expected_geometry(fixture: &AssociationFixture) -> (AssociationGeometry, FitMetrics) {
    let camera = &fixture.camera_matrix;
    let robot_to_local = robot_to_local(camera);
    let camera_to_local = (robot_to_local * robot_to_camera(camera).inverse())
        .inner
        .cast::<f64>();
    let origin = camera_to_local.translation.vector;
    let local: Vec<_> = fixture
        .expected
        .iter()
        .map(|e| {
            let ray = camera_to_local.rotation
                * camera
                    .intrinsics
                    .bearing(point![e.detection[0], e.detection[1]])
                    .inner
                    .cast::<f64>();
            assert!(origin.z > 0.0 && ray.z < 0.0);
            (origin - ray * origin.z / ray.z).xy()
        })
        .collect();
    let field: Vec<_> = fixture
        .expected
        .iter()
        .map(|e| Vector2::from(e.landmark).cast::<f64>())
        .collect();
    let count = local.len() as f64;
    let local_mean = local.iter().copied().sum::<Vector2<f64>>() / count;
    let field_mean = field.iter().copied().sum::<Vector2<f64>>() / count;
    let mut dot = 0.0;
    let mut cross = 0.0;
    let mut norm = 0.0;
    for (l, f) in local.iter().zip(&field) {
        let l = l - local_mean;
        let f = f - field_mean;
        dot += l.dot(&f);
        cross += l.x * f.y - l.y * f.x;
        norm += l.norm_squared();
    }
    let yaw = cross.atan2(dot);
    let rotation = UnitComplex::new(yaw);
    let translation = field_mean - rotation * local_mean;
    let metric_rms = (local
        .iter()
        .zip(&field)
        .map(|(l, f)| (rotation * l + translation - f).norm_squared())
        .sum::<f64>()
        / count)
        .sqrt();
    let alignment_3d = nalgebra::Isometry3::from_parts(
        Translation3::new(translation.x, translation.y, 0.0),
        UnitQuaternion::from_euler_angles(0.0, 0.0, yaw),
    );
    let field_to_camera = (alignment_3d * camera_to_local).inverse();
    let pixel_rms = (fixture
        .expected
        .iter()
        .map(|e| {
            let p =
                field_to_camera * nalgebra::point![e.landmark[0] as f64, e.landmark[1] as f64, 0.0];
            assert!(p.z > 0.0);
            let projected = camera
                .intrinsics
                .focals
                .cast::<f64>()
                .component_mul(&(p.coords.xy() / p.z))
                + camera.intrinsics.optical_center.inner.coords.cast::<f64>();
            (projected - Vector2::from(e.detection).cast::<f64>()).norm_squared()
        })
        .sum::<f64>()
        / count)
        .sqrt();
    // Isotropic planar least-squares covariance: 2N observations, three fitted parameters.
    // Centering decouples centroid translation from yaw. Transform their joint uncertainty
    // to the robot's right tangent, retaining the yaw/translation lever-arm correlations.
    assert!(count >= 3.0 && norm > 0.0);
    let variance = metric_rms.powi(2) * count / (2.0 * count - 3.0);
    let centered_covariance = nalgebra::Matrix3::from_diagonal(&nalgebra::vector![
        variance / count,
        variance / count,
        variance / norm,
    ]);
    let robot_to_field = alignment_3d * robot_to_local.inner.cast::<f64>();
    let field_to_robot = robot_to_field.rotation.inverse();
    let lever =
        rotation * (robot_to_local.inner.translation.vector.xy().cast::<f64>() - local_mean);
    let mut tangent_jacobian = nalgebra::SMatrix::<f64, 6, 3>::zeros();
    for axis in 0..2 {
        tangent_jacobian
            .fixed_view_mut::<3, 1>(3, axis)
            .copy_from(&(field_to_robot * Vector3::ith(axis, 1.0)));
    }
    tangent_jacobian
        .fixed_view_mut::<3, 1>(0, 2)
        .copy_from(&(field_to_robot * Vector3::z()));
    tangent_jacobian
        .fixed_view_mut::<3, 1>(3, 2)
        .copy_from(&(field_to_robot * Vector3::new(-lever.y, lever.x, 0.0)));
    let covariance = tangent_jacobian * centered_covariance * tangent_jacobian.transpose();
    let geometry = AssociationGeometry {
        epoch: 0,
        generation: 0,
        estimate: PoseEstimate {
            pose: Isometry3::wrap(robot_to_field),
            covariance,
        },
        last_successful_solve: Time::from_nanos(0),
    };
    (
        geometry,
        FitMetrics {
            camera_height: origin.z,
            free_height_scale: dot.hypot(cross) / norm,
            metric_rms,
            pixel_rms,
        },
    )
}
