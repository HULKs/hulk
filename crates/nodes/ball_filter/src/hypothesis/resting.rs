use std::time::Duration;

use coordinate_systems::Ground;
use filtering::kalman_filter::KalmanFilter;
use linear_algebra::Isometry2;
use nalgebra::Matrix2;
use types::multivariate_normal_distribution::MultivariateNormalDistribution;

pub(super) trait RestingPredict {
    fn predict(
        &mut self,
        delta_time: Duration,
        last_to_current_odometry: Isometry2<Ground, Ground>,
        process_noise: Matrix2<f32>,
    );
}

pub(super) trait RestingUpdate {
    fn update(&mut self, measurement: MultivariateNormalDistribution<2>);
}

impl RestingPredict for MultivariateNormalDistribution<2> {
    fn predict(
        &mut self,
        delta_time: Duration,
        last_to_current_odometry: Isometry2<Ground, Ground>,
        process_noise: Matrix2<f32>,
    ) {
        let rotation = last_to_current_odometry.inner.rotation.to_rotation_matrix();
        let translation = last_to_current_odometry.inner.translation.vector;

        KalmanFilter::predict(
            self,
            *rotation.matrix(),
            Matrix2::identity(),
            translation,
            // Covariance parameters retain their reference 2 ms (500 Hz) scale.
            process_noise * (delta_time.as_secs_f64() / 0.002) as f32,
        );
    }
}

impl RestingUpdate for MultivariateNormalDistribution<2> {
    fn update(&mut self, measurement: MultivariateNormalDistribution<2>) {
        KalmanFilter::update(
            self,
            Matrix2::identity(),
            measurement.mean,
            measurement.covariance,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::{Vector2, matrix};

    #[test]
    fn resting_noise_accumulation_agrees_at_100_250_500_hz_and_skipped_updates() {
        let noise = matrix![0.001, 0.0002; 0.0002, 0.002];
        let initial = MultivariateNormalDistribution {
            mean: Vector2::new(1.0, -2.0),
            covariance: Matrix2::identity() * 0.2,
        };
        let mut once = initial;
        RestingPredict::predict(
            &mut once,
            Duration::from_secs(1),
            Isometry2::identity(),
            noise,
        );
        for frequency in [100, 250, 500] {
            let mut repeated = initial;
            for _ in 0..frequency {
                RestingPredict::predict(
                    &mut repeated,
                    Duration::from_nanos(1_000_000_000 / frequency),
                    Isometry2::identity(),
                    noise,
                );
            }
            assert_eq!(repeated.mean, once.mean);
            assert!((repeated.covariance - once.covariance).norm() < 2e-5);
            assert!(repeated.covariance.symmetric_eigen().eigenvalues.min() > 0.0);
        }
    }

    #[test]
    fn zero_dt_adds_no_noise_but_still_transforms_odometry() {
        let mut distribution = MultivariateNormalDistribution {
            mean: Vector2::new(1.0, 2.0),
            covariance: matrix![0.2, 0.01; 0.01, 0.4],
        };
        let before = distribution;
        let noise = Matrix2::identity();
        RestingPredict::predict(
            &mut distribution,
            Duration::ZERO,
            Isometry2::identity(),
            noise,
        );
        assert_eq!(distribution.mean, before.mean);
        assert_eq!(distribution.covariance, before.covariance);
        let pose = Isometry2::from_parts(linear_algebra::vector![2.0, -1.0], 0.7);
        let rotation = pose.inner.rotation.to_rotation_matrix();
        RestingPredict::predict(&mut distribution, Duration::ZERO, pose, noise);
        assert!(
            (distribution.mean - (rotation * before.mean + pose.inner.translation.vector)).norm()
                < 1e-6
        );
        assert!(
            (distribution.covariance
                - rotation.matrix() * before.covariance * rotation.matrix().transpose())
            .norm()
                < 1e-6
        );
    }
}
