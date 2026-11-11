use std::time::Duration;

use nalgebra::{Matrix4, matrix};

use crate::motion_filter::BallMotionError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MotionTransitions {
    pub stationary_to_stationary: f64,
    pub stationary_to_rolling: f64,
    pub rolling_to_stationary: f64,
    pub rolling_to_rolling: f64,
}

/// Endpoint probabilities for the constant-rate two-state Markov chain.
pub(crate) fn transitions(
    start_rate: f32,
    stop_rate: f32,
    dt: Duration,
) -> Result<MotionTransitions, BallMotionError> {
    let start = f64::from(start_rate);
    let stop = f64::from(stop_rate);
    let total_rate = start + stop;
    if total_rate == 0.0 || dt.is_zero() {
        return Ok(MotionTransitions {
            stationary_to_stationary: 1.0,
            stationary_to_rolling: 0.0,
            rolling_to_stationary: 0.0,
            rolling_to_rolling: 1.0,
        });
    }
    let integrated_rate = total_rate * dt.as_secs_f64();
    let survival = (-integrated_rate).exp();
    let change = -(-integrated_rate).exp_m1();
    // Do not form a diagonal as 1 - off_diagonal: a one-way survival
    // probability can remain representable long after the latter rounds to 1.
    let transitions = MotionTransitions {
        stationary_to_stationary: survival + (stop / total_rate) * change,
        stationary_to_rolling: (start / total_rate) * change,
        rolling_to_stationary: (stop / total_rate) * change,
        rolling_to_rolling: survival + (start / total_rate) * change,
    };
    if !integrated_rate.is_finite()
        || [
            transitions.stationary_to_stationary,
            transitions.stationary_to_rolling,
            transitions.rolling_to_stationary,
            transitions.rolling_to_rolling,
        ]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(BallMotionError::NonFiniteDynamics);
    }
    Ok(transitions)
}

/// Transition matrix F and process covariance Q for the integrated linear-drag
/// model dp = v dt, dv = -drag v dt + √q dW, independently per axis.
/// Here q is the acceleration spectral density. With zero drag:
///
/// ```text
/// Qpp = q Δt³ / 3
/// Qpv = q Δt² / 2
/// Qvv = q Δt
/// ```
pub(crate) fn rolling(
    drag: f32,
    spectral_density: f32,
    dt: Duration,
) -> Result<(Matrix4<f32>, Matrix4<f32>), BallMotionError> {
    if dt.is_zero() {
        return Ok((Matrix4::identity(), Matrix4::zeros()));
    }
    let time = dt.as_secs_f64();
    let integrated_drag = f64::from(drag) * time;
    let (velocity_decay, position_gain, cross_noise_gain, position_noise_gain, velocity_noise_gain) =
        if integrated_drag == 0.0 {
            (1.0, 1.0, 0.5, 1.0 / 3.0, 1.0)
        } else {
            let velocity_decay = (-integrated_drag).exp();
            let velocity_loss = -(-integrated_drag).exp_m1();
            let squared_velocity_loss = -(-2.0 * integrated_drag).exp_m1();
            let position_gain = velocity_loss / integrated_drag;
            let cross_noise_gain = position_gain * position_gain / 2.0;
            let velocity_noise_gain = squared_velocity_loss / (2.0 * integrated_drag);
            let position_noise_gain = if integrated_drag <= 0.1 {
                // This gain c(x) gives Qpp = q Δt³ c(x), with x = drag Δt.
                // For small x (including short time steps), the closed-form numerator
                // subtracts terms of size x to obtain a result of size x³, losing
                // precision even with exp_m1. Use the Taylor series through x⁶,
                // evaluated in Horner form, to approach c(0) = 1/3 without cancellation.
                // For x ≤ 0.1, the truncation error is below 1.406e-11, well below
                // f32 precision for this coefficient.
                let x = integrated_drag;
                1.0 / 3.0
                    + x * (-1.0 / 4.0
                        + x * (7.0 / 60.0
                            + x * (-1.0 / 24.0
                                + x * (31.0 / 2520.0
                                    + x * (-1.0 / 320.0 + x * (127.0 / 181440.0))))))
            } else {
                (integrated_drag - 2.0 * velocity_loss + squared_velocity_loss / 2.0)
                    / integrated_drag.powi(3)
            };
            (
                velocity_decay,
                position_gain,
                cross_noise_gain,
                position_noise_gain,
                velocity_noise_gain,
            )
        };
    let position_from_velocity = time * position_gain;
    let transition = matrix![
        1.0, 0.0, position_from_velocity, 0.0;
        0.0, 1.0, 0.0, position_from_velocity;
        0.0, 0.0, velocity_decay, 0.0;
        0.0, 0.0, 0.0, velocity_decay;
    ];
    let noise = if spectral_density == 0.0 {
        Matrix4::<f64>::zeros()
    } else {
        let density = f64::from(spectral_density);
        let position_variance = density * time.powi(3) * position_noise_gain;
        let position_velocity_covariance = density * time.powi(2) * cross_noise_gain;
        let velocity_variance = density * time * velocity_noise_gain;
        matrix![
            position_variance, 0.0, position_velocity_covariance, 0.0;
            0.0, position_variance, 0.0, position_velocity_covariance;
            position_velocity_covariance, 0.0, velocity_variance, 0.0;
            0.0, position_velocity_covariance, 0.0, velocity_variance;
        ]
    };
    let transition = transition.cast::<f32>();
    let noise = noise.cast::<f32>();
    if !integrated_drag.is_finite()
        || transition
            .iter()
            .chain(noise.iter())
            .any(|value| !value.is_finite())
    {
        return Err(BallMotionError::NonFiniteDynamics);
    }
    Ok((transition, noise))
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn zero_and_tiny_drag_match_integrated_acceleration() {
        let expected_transition = matrix![
            1.0, 0.0, 2.0, 0.0;
            0.0, 1.0, 0.0, 2.0;
            0.0, 0.0, 1.0, 0.0;
            0.0, 0.0, 0.0, 1.0;
        ];
        let expected_noise = matrix![
            8.0, 0.0, 6.0, 0.0;
            0.0, 8.0, 0.0, 6.0;
            6.0, 0.0, 6.0, 0.0;
            0.0, 6.0, 0.0, 6.0;
        ];
        for drag in [0.0, 1e-20] {
            let (transition, noise) = rolling(drag, 3.0, Duration::from_secs(2)).unwrap();
            assert_relative_eq!(transition, expected_transition, epsilon = 1e-6);
            assert_relative_eq!(noise, expected_noise, epsilon = 1e-6);
        }
        assert_eq!(
            rolling(0.5, 3.0, Duration::ZERO).unwrap(),
            (Matrix4::identity(), Matrix4::zeros())
        );
        assert_eq!(
            rolling(0.5, 0.0, Duration::MAX).unwrap().1,
            Matrix4::zeros()
        );
    }

    #[test]
    fn constant_rolling_model_composes_across_small_drag_branch() {
        for drag in [0.0, 1e-10, 0.1, 0.10001, 2.0, 100.0] {
            let (first_f, first_q) = rolling(drag, 0.7, Duration::from_millis(350)).unwrap();
            let (second_f, second_q) = rolling(drag, 0.7, Duration::from_millis(650)).unwrap();
            let (whole_f, whole_q) = rolling(drag, 0.7, Duration::from_secs(1)).unwrap();
            assert_relative_eq!(second_f * first_f, whole_f, epsilon = 2e-7);
            assert_relative_eq!(
                second_f * first_q * second_f.transpose() + second_q,
                whole_q,
                epsilon = 2e-7
            );
        }
    }

    #[test]
    fn ctmc_zero_one_way_and_two_way_rates() {
        let identity = MotionTransitions {
            stationary_to_stationary: 1.0,
            stationary_to_rolling: 0.0,
            rolling_to_stationary: 0.0,
            rolling_to_rolling: 1.0,
        };
        assert_eq!(transitions(0.0, 0.0, Duration::MAX).unwrap(), identity);
        assert_eq!(transitions(1.0, 2.0, Duration::ZERO).unwrap(), identity);
        let one_way = transitions(100.0, 0.0, Duration::from_secs(1)).unwrap();
        assert_relative_eq!(
            one_way.stationary_to_stationary,
            (-100.0_f64).exp(),
            max_relative = 1e-15
        );
        assert_eq!(one_way.rolling_to_stationary, 0.0);
        assert_eq!(one_way.rolling_to_rolling, 1.0);
        let reverse = transitions(0.0, 100.0, Duration::from_secs(1)).unwrap();
        assert_eq!(reverse.stationary_to_stationary, 1.0);
        assert_eq!(reverse.stationary_to_rolling, 0.0);
        assert_eq!(reverse.rolling_to_rolling, one_way.stationary_to_stationary);
        let both = transitions(2.0, 3.0, Duration::from_millis(400)).unwrap();
        let changed = 1.0 - (-2.0_f64).exp();
        assert_relative_eq!(both.stationary_to_rolling, 0.4 * changed, epsilon = 1e-15);
        assert_relative_eq!(both.rolling_to_stationary, 0.6 * changed, epsilon = 1e-15);
        assert_relative_eq!(
            both.stationary_to_stationary + both.stationary_to_rolling,
            1.0,
            epsilon = 1e-15
        );
        assert_relative_eq!(
            both.rolling_to_stationary + both.rolling_to_rolling,
            1.0,
            epsilon = 1e-15
        );
    }
}
