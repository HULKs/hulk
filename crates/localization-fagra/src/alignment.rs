//! Planar alignment from camera bearings and known ground landmarks.

use fagra::EvaluationError;
use nalgebra::{Matrix2, Matrix2x3, Matrix3, Matrix4, SMatrix, Vector2, Vector3, Vector4};

use crate::finite;

/// Fit `field = translation + height * rotation * unit_ground_point`.
/// Unit-ground points are intersections of leveled rays with a plane one metre below
/// the camera. Translation is the camera's field XY; scale is camera height.
pub fn fit_ground_similarity(
    correspondences: impl ExactSizeIterator<Item = (Vector2<f64>, Vector2<f64>)> + Clone,
) -> Result<GroundPose, EvaluationError> {
    if correspondences.len() < 3 {
        return Err(EvaluationError::InvalidEvaluation);
    }
    let count = correspondences.len() as f64;
    let (mut q_mean, mut p_mean) = (Vector2::zeros(), Vector2::zeros());
    for (q, p) in correspondences.clone() {
        finite(q.iter().chain(p.iter()))?;
        q_mean += q;
        p_mean += p;
    }
    q_mean /= count;
    p_mean /= count;
    let (mut dot, mut cross, mut variance) = (0.0, 0.0, 0.0);
    let mut scatter = Matrix2::zeros();
    for (q, p) in correspondences {
        let q = q - q_mean;
        let p = p - p_mean;
        dot += q.dot(&p);
        cross += q.x * p.y - q.y * p.x;
        variance += q.norm_squared();
        scatter += q * q.transpose();
    }
    if variance <= 1.0e-12 || scatter.determinant() <= 1.0e-10 * variance * variance {
        return Err(EvaluationError::InvalidEvaluation);
    }
    let height = dot.hypot(cross) / variance;
    if !height.is_finite() || height <= 1.0e-6 {
        return Err(EvaluationError::InvalidEvaluation);
    }
    let rotation = nalgebra::UnitComplex::new(cross.atan2(dot));
    let translation = p_mean - rotation * q_mean * height;
    finite(translation.iter())?;
    Ok(GroundPose {
        position: translation,
        yaw: rotation.angle(),
        height,
    })
}

/// Camera XY, yaw and height above a ground plane; sensor tilt stays fixed.
#[derive(Clone, Copy, Debug)]
pub struct GroundPose {
    pub position: Vector2<f64>,
    pub yaw: f64,
    pub height: f64,
}

impl GroundPose {
    /// Minimal gravity-constrained hypothesis, not evidence of localization by itself.
    pub fn from_pair(
        ground: [Vector2<f64>; 2],
        field: [Vector2<f64>; 2],
        min_distance: f64,
    ) -> Option<Self> {
        if !min_distance.is_finite() || min_distance <= 0.0 {
            return None;
        }
        let a = ground[1] - ground[0];
        let b = field[1] - field[0];
        if a.norm() <= min_distance || b.norm() <= min_distance {
            return None;
        }
        let height = b.norm() / a.norm();
        let yaw = b.y.atan2(b.x) - a.y.atan2(a.x);
        let rotation = nalgebra::Rotation2::new(yaw);
        let pose = Self {
            position: field[0] - height * (rotation * ground[0]),
            yaw,
            height,
        };
        pose.is_valid().then_some(pose)
    }

    pub fn is_valid(self) -> bool {
        self.position.iter().all(|v| v.is_finite())
            && self.yaw.is_finite()
            && self.height.is_finite()
            && self.height > 0.0
    }
}

/// Projection and derivatives for the same four-unknown fit used by association and recovery.
pub struct GravityCamera {
    level_to_camera: Matrix3<f64>,
    focals: nalgebra::Vector2<f64>,
    center: nalgebra::Vector2<f64>,
    min_depth: f64,
}

pub struct GroundProjection {
    pub pixel: nalgebra::Vector2<f64>,
    /// Columns: camera x, y, yaw, height.
    pub jacobian: SMatrix<f64, 2, 4>,
    pub tilt_jacobian: Matrix2<f64>,
}

impl GravityCamera {
    pub fn new(
        camera_to_level: Matrix3<f64>,
        focals: nalgebra::Vector2<f64>,
        center: nalgebra::Vector2<f64>,
        min_depth: f64,
    ) -> Option<Self> {
        if !camera_to_level
            .iter()
            .chain(center.iter())
            .all(|v| v.is_finite())
            || !focals.iter().all(|v| v.is_finite() && *v > 0.0)
            || !min_depth.is_finite()
            || min_depth <= 0.0
            || (camera_to_level.transpose() * camera_to_level - Matrix3::identity()).norm() > 1.0e-5
            || camera_to_level.determinant() <= 0.0
        {
            return None;
        }
        Some(Self {
            level_to_camera: camera_to_level.transpose(),
            focals,
            center,
            min_depth,
        })
    }

    pub fn project(
        &self,
        pose: GroundPose,
        field: nalgebra::Vector2<f64>,
    ) -> Option<GroundProjection> {
        let (sin, cos) = pose.yaw.sin_cos();
        let offset = field - pose.position;
        let level = Vector3::new(
            cos * offset.x + sin * offset.y,
            -sin * offset.x + cos * offset.y,
            -pose.height,
        );
        let camera = self.level_to_camera * level;
        if !camera.iter().all(|v| v.is_finite()) || camera.z <= self.min_depth {
            return None;
        }
        let projection = Matrix2x3::new(
            self.focals.x / camera.z,
            0.0,
            -self.focals.x * camera.x / camera.z.powi(2),
            0.0,
            self.focals.y / camera.z,
            -self.focals.y * camera.y / camera.z.powi(2),
        ) * self.level_to_camera;
        let derivatives = SMatrix::<f64, 3, 4>::from_columns(&[
            Vector3::new(-cos, sin, 0.0),
            Vector3::new(-sin, -cos, 0.0),
            Vector3::new(level.y, -level.x, 0.0),
            -Vector3::z(),
        ]);
        Some(GroundProjection {
            pixel: self.focals.component_mul(&(camera.xy() / camera.z)) + self.center,
            jacobian: projection * derivatives,
            tilt_jacobian: Matrix2::from_columns(&[
                projection * Vector3::x().cross(&level),
                projection * Vector3::y().cross(&level),
            ]),
        })
    }

    /// Fixed correspondences: (field XY, detected pixel). No assignment or outlier changes.
    pub fn squared_error(
        &self,
        pose: GroundPose,
        observations: &[(nalgebra::Vector2<f64>, nalgebra::Vector2<f64>)],
    ) -> Option<f64> {
        observations
            .iter()
            .try_fold(0.0, |sum, (field, pixel)| {
                let error = self.project(pose, *field)?.pixel - pixel;
                Some(sum + error.norm_squared())
            })
            .filter(|error| error.is_finite())
    }

    /// At most eight damped Gauss–Newton steps, each with a fixed-size 4x4 solve.
    /// Rejected trials never replace the caller's seed.
    pub fn refine(
        &self,
        mut pose: GroundPose,
        observations: &[(nalgebra::Vector2<f64>, nalgebra::Vector2<f64>)],
        valid: impl Fn(GroundPose) -> bool,
    ) -> Option<GroundPose> {
        if observations.len() < 3 || !pose.is_valid() || !valid(pose) {
            return None;
        }
        let mut cost = self.squared_error(pose, observations)?;
        for _ in 0..8 {
            let mut information = Matrix4::zeros();
            let mut rhs = Vector4::zeros();
            for (field, pixel) in observations {
                let projection = self.project(pose, *field)?;
                information += projection.jacobian.transpose() * projection.jacobian;
                rhs -= projection.jacobian.transpose() * (projection.pixel - pixel);
            }
            let mut accepted = None;
            for damping in [1.0e-5, 1.0e-3, 0.1, 10.0] {
                let mut damped = information;
                for i in 0..4 {
                    damped[(i, i)] += damping * information[(i, i)].max(1.0e-6);
                }
                let Some(delta) = damped.cholesky().map(|factor| factor.solve(&rhs)) else {
                    continue;
                };
                let trial = GroundPose {
                    position: pose.position + delta.xy(),
                    yaw: pose.yaw + delta.z,
                    height: pose.height + delta.w,
                };
                if !trial.is_valid() || !valid(trial) {
                    continue;
                }
                if let Some(error) = self.squared_error(trial, observations)
                    && error < cost
                {
                    accepted = Some((trial, error, delta.norm_squared()));
                    break;
                }
            }
            let Some((trial, error, step)) = accepted else {
                break;
            };
            pose = trial;
            cost = error;
            if step < 1.0e-10 {
                break;
            }
        }
        Some(pose)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_projection_derivatives_and_pair_seed_are_consistent() {
        let camera = GravityCamera::new(
            nalgebra::UnitQuaternion::from_euler_angles(2.8, -0.2, -0.1)
                .to_rotation_matrix()
                .into_inner(),
            nalgebra::Vector2::new(210.0, 205.0),
            nalgebra::Vector2::new(250.0, 225.0),
            0.01,
        )
        .unwrap();
        let pose = GroundPose {
            position: nalgebra::Vector2::new(-2.0, 0.3),
            yaw: 0.2,
            height: 0.8,
        };
        let field = nalgebra::Vector2::new(3.0, 1.0);
        let actual = camera.project(pose, field).unwrap();
        for i in 0..4 {
            let shifted = |step| {
                let mut p = pose;
                match i {
                    0 => p.position.x += step,
                    1 => p.position.y += step,
                    2 => p.yaw += step,
                    _ => p.height += step,
                }
                camera.project(p, field).unwrap().pixel
            };
            let numerical = (shifted(1.0e-6) - shifted(-1.0e-6)) / 2.0e-6;
            assert!((numerical - actual.jacobian.column(i)).norm() < 1.0e-4);
        }
        for (i, axis) in [Vector3::x(), Vector3::y()].into_iter().enumerate() {
            let tilted = |step| {
                let rotation = nalgebra::UnitQuaternion::from_scaled_axis(axis * step)
                    .to_rotation_matrix()
                    .into_inner()
                    * camera.level_to_camera.transpose();
                GravityCamera::new(rotation, camera.focals, camera.center, camera.min_depth)
                    .unwrap()
                    .project(pose, field)
                    .unwrap()
                    .pixel
            };
            let numerical = (tilted(-1.0e-6) - tilted(1.0e-6)) / 2.0e-6;
            assert!((numerical - actual.tilt_jacobian.column(i)).norm() < 1.0e-4);
        }
        let ground = [
            nalgebra::Vector2::new(1.0, -0.5),
            nalgebra::Vector2::new(2.0, 0.7),
        ];
        let rotation = nalgebra::Rotation2::new(pose.yaw);
        let field = ground.map(|q| pose.position + pose.height * (rotation * q));
        let seed = GroundPose::from_pair(ground, field, 1.0e-6).unwrap();
        assert!((seed.position - pose.position).norm() < 1.0e-10);
        assert!((seed.yaw - pose.yaw).abs() < 1.0e-10);
        assert!((seed.height - pose.height).abs() < 1.0e-10);
        assert!(GroundPose::from_pair([ground[0]; 2], field, 1.0e-6).is_none());
        assert!(GroundPose::from_pair(ground, field, f64::NAN).is_none());
        assert!(
            GravityCamera::new(
                Matrix3::zeros(),
                nalgebra::Vector2::repeat(210.0),
                nalgebra::Vector2::zeros(),
                0.01
            )
            .is_none()
        );
    }

    #[test]
    fn similarity_recovers_height_yaw_and_position_and_rejects_degeneracy() {
        let expected = nalgebra::Isometry2::new(nalgebra::Vector2::new(-2.0, 1.0), 0.7);
        for height in [0.4_f64, 0.9, 2.0] {
            let points = [(-1.0, -1.0), (1.0, -1.0), (0.0, 1.0), (0.3, 0.7)].map(|(x, y)| {
                let q = nalgebra::Vector2::new(x, y);
                (q, (expected * nalgebra::Point2::from(q * height)).coords)
            });
            for count in [3, 4] {
                let fit = fit_ground_similarity(points[..count].iter().copied()).unwrap();
                assert!((fit.height - height).abs() < 1e-10);
                assert!((fit.position - expected.translation.vector).norm() < 1e-10);
                assert!((fit.yaw - expected.rotation.angle()).abs() < 1e-10);
            }
            assert!(fit_ground_similarity([points[0]; 3].into_iter()).is_err());
        }
        let line = [0.0, 1.0, 2.0].map(|x| (Vector2::new(x, 0.0), Vector2::new(x, 0.0)));
        assert!(fit_ground_similarity(line.into_iter()).is_err());
    }
}
