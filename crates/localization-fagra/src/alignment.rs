//! Planar alignment from camera bearings and known ground landmarks.

use coordinate_systems::Field;
use fagra::EvaluationError;
use linear_algebra::{Point2, Rotation2, Vector2};
use nalgebra::RealField;

use crate::{finite, variables::rotation::scalar};

/// Fit `field = translation + height * rotation * unit_ground_point`.
/// Unit-ground points are intersections of leveled rays with a plane one metre below
/// the camera. Translation is the camera's field XY; scale is camera height.
pub struct GroundSimilarity<Level, R: RealField + Copy> {
    pub camera_position: Point2<Field, R>,
    pub rotation: Rotation2<Level, Field, R>,
    pub camera_height: R,
}

pub fn fit_ground_similarity<Level, R: RealField + Copy>(
    correspondences: impl ExactSizeIterator<Item = (Vector2<Level, R>, Point2<Field, R>)> + Clone,
) -> Result<GroundSimilarity<Level, R>, EvaluationError> {
    if correspondences.len() < 3 {
        return Err(EvaluationError::InvalidEvaluation);
    }
    let count = scalar::<R>(correspondences.len() as f64);
    let (mut q_mean, mut p_mean) = (
        nalgebra::Vector2::<R>::zeros(),
        nalgebra::Vector2::<R>::zeros(),
    );
    for (q, p) in correspondences.clone() {
        finite(q.inner.iter().chain(p.inner.coords.iter()))?;
        q_mean += q.inner;
        p_mean += p.inner.coords;
    }
    q_mean /= count;
    p_mean /= count;
    let (mut dot, mut cross, mut variance) = (R::zero(), R::zero(), R::zero());
    let mut scatter = nalgebra::Matrix2::<R>::zeros();
    for (q, p) in correspondences {
        let q = q.inner - q_mean;
        let p = p.inner.coords - p_mean;
        dot += q.dot(&p);
        cross += q.x * p.y - q.y * p.x;
        variance += q.norm_squared();
        scatter += q * q.transpose();
    }
    if variance <= scalar(1.0e-12)
        || scatter.determinant() <= scalar::<R>(1.0e-10) * variance * variance
    {
        return Err(EvaluationError::InvalidEvaluation);
    }
    let height = dot.hypot(cross) / variance;
    if !height.is_finite() || height <= scalar(1.0e-6) {
        return Err(EvaluationError::InvalidEvaluation);
    }
    let rotation = nalgebra::UnitComplex::new(cross.atan2(dot));
    let translation = p_mean - rotation * q_mean * height;
    finite(translation.iter())?;
    Ok(GroundSimilarity {
        camera_position: Point2::wrap(translation.into()),
        rotation: Rotation2::wrap(rotation),
        camera_height: height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use coordinate_systems::Local;

    #[test]
    fn similarity_recovers_height_yaw_and_position_and_rejects_degeneracy() {
        let expected = nalgebra::Isometry2::new(nalgebra::Vector2::new(-2.0, 1.0), 0.7);
        for height in [0.4_f64, 0.9, 2.0] {
            let points = [(-1.0, -1.0), (1.0, -1.0), (0.0, 1.0), (0.3, 0.7)].map(|(x, y)| {
                let q = nalgebra::Vector2::new(x, y);
                (
                    Vector2::<Local, f64>::wrap(q),
                    Point2::wrap(expected * nalgebra::Point2::from(q * height)),
                )
            });
            for count in [3, 4] {
                let fit = fit_ground_similarity(points[..count].iter().copied()).unwrap();
                assert!((fit.camera_height - height).abs() < 1e-10);
                assert!(
                    (fit.camera_position.inner.coords - expected.translation.vector).norm() < 1e-10
                );
                assert!((fit.rotation.angle() - expected.rotation.angle()).abs() < 1e-10);
            }
            assert!(fit_ground_similarity([points[0]; 3].into_iter()).is_err());
        }
        let line = [0.0, 1.0, 2.0].map(|x| {
            (
                Vector2::<Local, f64>::wrap(nalgebra::Vector2::new(x, 0.0)),
                Point2::wrap(nalgebra::Point2::new(x, 0.0)),
            )
        });
        assert!(fit_ground_similarity(line.into_iter()).is_err());
    }
}
