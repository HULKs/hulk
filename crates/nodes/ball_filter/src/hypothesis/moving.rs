use std::time::Duration;

use coordinate_systems::Ground;
use filtering::kalman_filter::KalmanFilter;
use linear_algebra::Isometry2;
use nalgebra::{Matrix2x4, Matrix4, Matrix4x2, matrix};
use types::multivariate_normal_distribution::MultivariateNormalDistribution;

pub(super) trait MovingPredict {
    fn predict(
        &mut self,
        delta_time: Duration,
        last_to_current_odometry: Isometry2<Ground, Ground>,
        velocity_decay: f32,
        process_noise: Matrix4<f32>,
    );
}

pub(super) trait MovingUpdate {
    fn update(&mut self, measurement: MultivariateNormalDistribution<2>);
}

impl MovingPredict for MultivariateNormalDistribution<4> {
    fn predict(
        &mut self,
        delta_time: Duration,
        last_to_current_odometry: Isometry2<Ground, Ground>,
        velocity_decay: f32,
        process_noise: Matrix4<f32>,
    ) {
        let (constant_velocity_prediction, process_noise) =
            elapsed_time_model(delta_time, velocity_decay, process_noise);

        let rotation = last_to_current_odometry.inner.rotation.to_rotation_matrix();
        let rotation = rotation.matrix();
        let translation = last_to_current_odometry.inner.translation.vector;

        let state_rotation = matrix![
            rotation.m11, rotation.m12, 0.0, 0.0;
            rotation.m21, rotation.m22, 0.0, 0.0;
            0.0, 0.0, rotation.m11, rotation.m12;
            0.0, 0.0, rotation.m21, rotation.m22;
        ];

        let state_prediction = constant_velocity_prediction * state_rotation;
        KalmanFilter::predict(
            self,
            state_prediction,
            Matrix4x2::identity(),
            translation,
            process_noise,
        );
    }
}

// Existing parameters describe one 2 ms (500 Hz) interval. Interpret their
// covariance as a continuous noise intensity Q / 2 ms and integrate it through
// dx/dt = v, dv/dt = -lambda v. The velocity multiplier at 2 ms remains exactly
// velocity_decay; integrating position during decay changes the old Euler step
// by about 0.1% for the default 0.998. Noise covariance gains the corresponding
// position/velocity cross terms instead of depending on callback frequency.
const REFERENCE_SECONDS: f64 = 0.002;

fn elapsed_time_model(
    delta_time: Duration,
    velocity_decay: f32,
    process_noise: Matrix4<f32>,
) -> (Matrix4<f32>, Matrix4<f32>) {
    let dt = delta_time.as_secs_f64();
    if dt == 0.0 {
        return (Matrix4::identity(), Matrix4::zeros());
    }
    // Zero is the well-defined infinite-damping limit: no displacement from
    // velocity, and no accumulated velocity uncertainty. No artificial floor
    // is introduced. The caller can recognize this zero-velocity resting state.
    let (decay, integral_a, integral_b, integral_aa, integral_bb) = if velocity_decay == 0.0 {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        let lambda = -f64::from(velocity_decay).ln() / REFERENCE_SECONDS;
        let x = lambda * dt;
        let decay = (-x).exp();
        let integral_a = if x == 0.0 {
            dt
        } else {
            dt * -(-x).exp_m1() / x
        };
        let integral_aa = if x == 0.0 {
            dt
        } else {
            dt * -(-2.0 * x).exp_m1() / (2.0 * x)
        };
        // Avoid subtracting almost equal exponentials near decay=1. These
        // series integrate b(s)=(1-exp(-lambda*s))/lambda and b(s)^2.
        let (integral_b, integral_bb) = if x.abs() < 0.01 {
            (
                dt * dt
                    * (0.5 + x * (-1.0 / 6.0 + x * (1.0 / 24.0 + x * (-1.0 / 120.0 + x / 720.0)))),
                dt * dt
                    * dt
                    * (1.0 / 3.0
                        + x * (-1.0 / 4.0
                            + x * (7.0 / 60.0 + x * (-1.0 / 24.0 + x * 31.0 / 2520.0)))),
            )
        } else {
            (
                (dt - integral_a) / lambda,
                (dt - 2.0 * integral_a + integral_aa) / (lambda * lambda),
            )
        };
        (decay, integral_a, integral_b, integral_aa, integral_bb)
    };
    let integral_ab = integral_a * integral_a / 2.0;
    let noise = process_noise.cast::<f64>();
    // Exact integral of F(s) Q F(s)^T, including all supplied off-diagonal
    // covariance entries. This preserves PSD for a PSD input covariance.
    let integrated_noise = Matrix4::<f64>::from_fn(|row, column| {
        let value = match (row < 2, column < 2) {
            (true, true) => {
                dt * noise[(row, column)]
                    + integral_b * (noise[(row + 2, column)] + noise[(row, column + 2)])
                    + integral_bb * noise[(row + 2, column + 2)]
            }
            (true, false) => {
                integral_a * noise[(row, column)] + integral_ab * noise[(row + 2, column)]
            }
            (false, true) => {
                integral_a * noise[(row, column)] + integral_ab * noise[(row, column + 2)]
            }
            (false, false) => integral_aa * noise[(row, column)],
        };
        value / REFERENCE_SECONDS
    });
    let prediction = matrix![
        1.0, 0.0, integral_a as f32, 0.0;
        0.0, 1.0, 0.0, integral_a as f32;
        0.0, 0.0, decay as f32, 0.0;
        0.0, 0.0, 0.0, decay as f32;
    ];
    (prediction, integrated_noise.cast::<f32>())
}

impl MovingUpdate for MultivariateNormalDistribution<4> {
    fn update(&mut self, measurement: MultivariateNormalDistribution<2>) {
        KalmanFilter::update(
            self,
            Matrix2x4::identity(),
            measurement.mean,
            measurement.covariance,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::Vector4;

    fn correlated_noise() -> Matrix4<f32> {
        let root = matrix![
            0.02, 0.0, 0.0, 0.0;
            0.004, 0.03, 0.0, 0.0;
            0.008, -0.003, 0.04, 0.0;
            0.002, 0.005, 0.006, 0.025;
        ];
        root * root.transpose()
    }

    fn initial() -> MultivariateNormalDistribution<4> {
        MultivariateNormalDistribution {
            mean: Vector4::new(1.0, -2.0, 3.0, -1.0),
            covariance: Matrix4::identity() * 0.2,
        }
    }

    fn assert_close(a: &MultivariateNormalDistribution<4>, b: &MultivariateNormalDistribution<4>) {
        assert!((a.mean - b.mean).norm() < 2e-4 * (1.0 + b.mean.norm()));
        assert!((a.covariance - b.covariance).norm() < 2e-4 * (1.0 + b.covariance.norm()));
    }

    #[test]
    fn moving_prediction_agrees_at_100_250_500_hz_and_after_skipped_updates() {
        let noise = correlated_noise();
        let mut once = initial();
        MovingPredict::predict(
            &mut once,
            Duration::from_secs(1),
            Isometry2::identity(),
            0.998,
            noise,
        );
        for frequency in [100, 250, 500] {
            let mut repeated = initial();
            let dt = Duration::from_nanos(1_000_000_000 / frequency);
            for _ in 0..frequency {
                MovingPredict::predict(&mut repeated, dt, Isometry2::identity(), 0.998, noise);
            }
            assert_close(&repeated, &once);
        }
    }

    #[test]
    fn transition_and_full_noise_covariance_compose_for_fractional_intervals() {
        let noise = correlated_noise();
        for decay in [0.5, 0.998, 1.0 - f32::EPSILON, 1.0] {
            for micros in [1, 1000, 2000, 10_000, 370_000] {
                let first = Duration::from_micros(micros);
                let second = Duration::from_micros(micros * 3 + 17);
                let (f1, q1) = elapsed_time_model(first, decay, noise);
                let (f2, q2) = elapsed_time_model(second, decay, noise);
                let (combined_f, combined_q) = elapsed_time_model(first + second, decay, noise);
                assert!((combined_f - f2 * f1).norm() < 1e-5 * (1.0 + combined_f.norm()));
                assert!(
                    (combined_q - (f2 * q1 * f2.transpose() + q2)).norm()
                        < 1e-5 * (1.0 + combined_q.norm())
                );
                assert!((combined_q - combined_q.transpose()).norm() < 1e-6);
                assert!(
                    combined_q.symmetric_eigen().eigenvalues.min()
                        >= -1e-6 * (1.0 + combined_q.norm())
                );
            }
        }
    }

    #[test]
    fn default_two_millisecond_velocity_decay_is_preserved() {
        let (transition, _) = elapsed_time_model(Duration::from_millis(2), 0.998, Matrix4::zeros());
        assert_eq!(transition[(2, 2)], 0.998);
        assert_eq!(transition[(3, 3)], 0.998);
        // Position now integrates damping rather than taking a forward Euler step.
        assert!(0.001_997 < transition[(0, 2)] && transition[(0, 2)] < 0.002);
        let (undamped, _) = elapsed_time_model(Duration::from_secs(1), 1.0, Matrix4::zeros());
        assert_eq!(undamped[(0, 2)], 1.0);
        assert_eq!(undamped[(2, 2)], 1.0);
    }

    #[test]
    fn undamped_velocity_noise_integrates_to_brownian_position_covariance() {
        let noise = Matrix4::from_diagonal(&Vector4::new(0.0, 0.0, 0.02, 0.04));
        let (_, integrated) = elapsed_time_model(Duration::from_millis(100), 1.0, noise);
        // 0.02 per 2 ms = 10 (m/s)^2/s. Brownian velocity over 0.1 s
        // has Var(v)=1, Cov(x,v)=0.05, and Var(x)=0.01/3.
        assert!((integrated[(2, 2)] - 1.0).abs() < 1e-7);
        assert!((integrated[(0, 2)] - 0.05).abs() < 1e-7);
        assert!((integrated[(0, 0)] - 0.01 / 3.0).abs() < 1e-7);
        assert!((integrated[(3, 3)] - 2.0).abs() < 1e-7);
        assert_eq!(integrated[(0, 1)], 0.0);
    }

    #[test]
    fn zero_dt_only_changes_coordinates_and_preserves_noise_free_state() {
        let mut untouched = initial();
        MovingPredict::predict(
            &mut untouched,
            Duration::ZERO,
            Isometry2::identity(),
            0.998,
            correlated_noise(),
        );
        assert_eq!(untouched.mean, initial().mean);
        assert_eq!(untouched.covariance, initial().covariance);

        let pose = Isometry2::from_parts(linear_algebra::vector![1.0, -2.0], 0.7);
        let rotation = pose.inner.rotation.to_rotation_matrix();
        let mut state_rotation = Matrix4::zeros();
        state_rotation
            .fixed_view_mut::<2, 2>(0, 0)
            .copy_from(rotation.matrix());
        state_rotation
            .fixed_view_mut::<2, 2>(2, 2)
            .copy_from(rotation.matrix());
        let noise = correlated_noise();
        let mut separate = initial();
        MovingPredict::predict(
            &mut separate,
            Duration::from_millis(137),
            Isometry2::identity(),
            0.998,
            noise,
        );
        MovingPredict::predict(&mut separate, Duration::ZERO, pose, 0.998, noise);
        let mut together = initial();
        // Noise is supplied in the output Ground frame, as in the original API.
        MovingPredict::predict(
            &mut together,
            Duration::from_millis(137),
            pose,
            0.998,
            state_rotation * noise * state_rotation.transpose(),
        );
        assert_close(&together, &separate);
    }

    #[test]
    fn zero_decay_is_the_psd_instantaneous_resting_limit() {
        let (transition, noise) =
            elapsed_time_model(Duration::from_millis(2), 0.0, correlated_noise());
        assert_eq!(
            transition.fixed_view::<2, 2>(2, 2),
            nalgebra::Matrix2::zeros()
        );
        assert_eq!(noise.fixed_view::<2, 2>(2, 2), nalgebra::Matrix2::zeros());
        assert!(noise.symmetric_eigen().eigenvalues.min() >= 0.0);
    }
}
