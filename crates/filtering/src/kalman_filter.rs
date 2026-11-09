use nalgebra::{Cholesky, Const, SMatrix, SVector};
use thiserror::Error;

use crate::multivariate_normal_distribution::MultivariateNormalDistribution;

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum KalmanError {
    #[error("non-finite Kalman input")]
    NonFiniteInput,
    #[error("innovation covariance is not positive definite")]
    InvalidInnovationCovariance,
    #[error("non-finite Kalman result")]
    NonFiniteResult,
}

/// Kalman operations on a Gaussian. Covariance inputs must be symmetric positive
/// semidefinite; this is a caller precondition, not checked by these operations.
/// Singular covariances are allowed if the innovation covariance is positive definite.
/// No covariance repair is performed. Mutating operations leave the Gaussian
/// unchanged on error.
pub trait KalmanFilter<const STATE_DIMENSION: usize> {
    fn predict<const CONTROL_DIMENSION: usize>(
        &mut self,
        state_prediction: SMatrix<f32, STATE_DIMENSION, STATE_DIMENSION>,
        control_input_model: SMatrix<f32, STATE_DIMENSION, CONTROL_DIMENSION>,
        control: SVector<f32, CONTROL_DIMENSION>,
        process_noise: SMatrix<f32, STATE_DIMENSION, STATE_DIMENSION>,
    ) -> Result<(), KalmanError>;

    fn evaluate<const MEASUREMENT_DIMENSION: usize>(
        &self,
        measurement_prediction: SMatrix<f32, MEASUREMENT_DIMENSION, STATE_DIMENSION>,
        measurement: SVector<f32, MEASUREMENT_DIMENSION>,
        measurement_noise: SMatrix<f32, MEASUREMENT_DIMENSION, MEASUREMENT_DIMENSION>,
    ) -> Result<MeasurementEvaluation<'_, STATE_DIMENSION, MEASUREMENT_DIMENSION>, KalmanError>;

    fn update<const MEASUREMENT_DIMENSION: usize>(
        &mut self,
        measurement_prediction: SMatrix<f32, MEASUREMENT_DIMENSION, STATE_DIMENSION>,
        measurement: SVector<f32, MEASUREMENT_DIMENSION>,
        measurement_noise: SMatrix<f32, MEASUREMENT_DIMENSION, MEASUREMENT_DIMENSION>,
    ) -> Result<(), KalmanError>;
}

/// A measurement evaluated against its immutably borrowed prior. Dropping an
/// evaluation rejects the measurement without changing the prior; conditioning
/// consumes it and returns an owned posterior.
#[derive(Debug)]
pub struct MeasurementEvaluation<
    'a,
    const STATE_DIMENSION: usize,
    const MEASUREMENT_DIMENSION: usize,
> {
    prior: &'a MultivariateNormalDistribution<STATE_DIMENSION>,
    measurement_prediction: SMatrix<f32, MEASUREMENT_DIMENSION, STATE_DIMENSION>,
    measurement_noise: SMatrix<f32, MEASUREMENT_DIMENSION, MEASUREMENT_DIMENSION>,
    covariance_times_measurement_transpose: SMatrix<f32, STATE_DIMENSION, MEASUREMENT_DIMENSION>,
    innovation: SVector<f32, MEASUREMENT_DIMENSION>,
    innovation_factor: Cholesky<f32, Const<MEASUREMENT_DIMENSION>>,
    squared_mahalanobis_distance: f32,
    log_likelihood: f32,
}

impl<const N: usize, const M: usize> MeasurementEvaluation<'_, N, M> {
    pub fn innovation(&self) -> &SVector<f32, M> {
        &self.innovation
    }

    pub fn squared_mahalanobis_distance(&self) -> f32 {
        self.squared_mahalanobis_distance
    }

    /// Natural logarithm of the normalized Gaussian measurement density.
    pub fn log_likelihood(&self) -> f32 {
        self.log_likelihood
    }

    pub fn condition(self) -> Result<MultivariateNormalDistribution<N>, KalmanError> {
        let gain = self
            .innovation_factor
            .solve(&self.covariance_times_measurement_transpose.transpose())
            .transpose();
        let mean = self.prior.mean + gain * self.innovation;
        let attenuation = SMatrix::<f32, N, N>::identity() - gain * self.measurement_prediction;
        let covariance = attenuation * self.prior.covariance * attenuation.transpose()
            + gain * self.measurement_noise * gain.transpose();
        if gain
            .iter()
            .chain(attenuation.iter())
            .chain(mean.iter())
            .chain(covariance.iter())
            .any(|x| !x.is_finite())
        {
            return Err(KalmanError::NonFiniteResult);
        }
        Ok(MultivariateNormalDistribution { mean, covariance })
    }
}

impl<const N: usize> KalmanFilter<N> for MultivariateNormalDistribution<N> {
    fn predict<const C: usize>(
        &mut self,
        state_prediction: SMatrix<f32, N, N>,
        control_input_model: SMatrix<f32, N, C>,
        control: SVector<f32, C>,
        process_noise: SMatrix<f32, N, N>,
    ) -> Result<(), KalmanError> {
        if self
            .mean
            .iter()
            .chain(self.covariance.iter())
            .chain(state_prediction.iter())
            .chain(control_input_model.iter())
            .chain(control.iter())
            .chain(process_noise.iter())
            .any(|x| !x.is_finite())
        {
            return Err(KalmanError::NonFiniteInput);
        }
        let mean = state_prediction * self.mean + control_input_model * control;
        let covariance =
            state_prediction * self.covariance * state_prediction.transpose() + process_noise;
        if mean.iter().chain(covariance.iter()).any(|x| !x.is_finite()) {
            return Err(KalmanError::NonFiniteResult);
        }
        *self = Self { mean, covariance };
        Ok(())
    }

    fn evaluate<const M: usize>(
        &self,
        measurement_prediction: SMatrix<f32, M, N>,
        measurement: SVector<f32, M>,
        measurement_noise: SMatrix<f32, M, M>,
    ) -> Result<MeasurementEvaluation<'_, N, M>, KalmanError> {
        if self
            .mean
            .iter()
            .chain(self.covariance.iter())
            .chain(measurement_prediction.iter())
            .chain(measurement.iter())
            .chain(measurement_noise.iter())
            .any(|x| !x.is_finite())
        {
            return Err(KalmanError::NonFiniteInput);
        }
        let innovation = measurement - measurement_prediction * self.mean;
        let covariance_times_measurement_transpose =
            self.covariance * measurement_prediction.transpose();
        let innovation_covariance =
            measurement_prediction * covariance_times_measurement_transpose + measurement_noise;
        if innovation
            .iter()
            .chain(covariance_times_measurement_transpose.iter())
            .chain(innovation_covariance.iter())
            .any(|x| !x.is_finite())
        {
            return Err(KalmanError::NonFiniteResult);
        }
        let innovation_factor = innovation_covariance
            .cholesky()
            .ok_or(KalmanError::InvalidInnovationCovariance)?;
        if innovation_factor.l_dirty().iter().any(|x| !x.is_finite()) {
            return Err(KalmanError::NonFiniteResult);
        }
        let whitened = innovation_factor
            .l_dirty()
            .solve_lower_triangular(&innovation)
            .ok_or(KalmanError::NonFiniteResult)?;
        let squared_mahalanobis_distance = whitened.norm_squared();
        // Sum logs of the diagonal directly: squaring the factor's entries or
        // forming a determinant first can overflow/underflow for finite inputs.
        let log_determinant = 2.0
            * (0..M)
                .map(|i| innovation_factor.l_dirty()[(i, i)].ln())
                .sum::<f32>();
        let log_likelihood = -0.5
            * (M as f32 * (2.0 * std::f32::consts::PI).ln()
                + log_determinant
                + squared_mahalanobis_distance);
        if whitened.iter().any(|x| !x.is_finite())
            || !squared_mahalanobis_distance.is_finite()
            || !log_determinant.is_finite()
            || !log_likelihood.is_finite()
        {
            return Err(KalmanError::NonFiniteResult);
        }
        Ok(MeasurementEvaluation {
            prior: self,
            measurement_prediction,
            measurement_noise,
            covariance_times_measurement_transpose,
            innovation,
            innovation_factor,
            squared_mahalanobis_distance,
            log_likelihood,
        })
    }

    fn update<const M: usize>(
        &mut self,
        measurement_prediction: SMatrix<f32, M, N>,
        measurement: SVector<f32, M>,
        measurement_noise: SMatrix<f32, M, M>,
    ) -> Result<(), KalmanError> {
        *self = self
            .evaluate(measurement_prediction, measurement, measurement_noise)?
            .condition()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use nalgebra::{Matrix1, Matrix2, SMatrix, Vector1, vector};

    use super::*;

    #[test]
    fn conditioning_matches_analytic_posterior_and_convenience_update() {
        let prior = MultivariateNormalDistribution {
            mean: vector![1.0, 2.0],
            covariance: Matrix2::new(4.0, 1.0, 1.0, 3.0),
        };
        let h = SMatrix::<f32, 1, 2>::new(1.0, 0.0);
        let measurement = vector![3.0];
        let noise = Matrix1::new(2.0);
        let posterior = prior
            .evaluate(h, measurement, noise)
            .unwrap()
            .condition()
            .unwrap();
        assert_relative_eq!(
            posterior.mean,
            vector![7.0 / 3.0, 7.0 / 3.0],
            epsilon = 1e-6
        );
        assert_relative_eq!(
            posterior.covariance,
            Matrix2::new(4.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0, 17.0 / 6.0),
            epsilon = 1e-6
        );
        let mut updated = prior;
        updated.update(h, measurement, noise).unwrap();
        assert_eq!(updated, posterior);
    }

    #[test]
    fn candidate_evaluations_preserve_prior_and_match_correlated_gaussian() {
        let prior = MultivariateNormalDistribution {
            mean: vector![0.0, 0.0],
            covariance: Matrix2::new(2.0, 1.0, 1.0, 2.0),
        };
        let original = prior;
        let candidate = prior
            .evaluate(Matrix2::identity(), vector![2.0, -1.0], Matrix2::identity())
            .unwrap();
        let distant = prior
            .evaluate(
                Matrix2::identity(),
                vector![20.0, -10.0],
                Matrix2::identity(),
            )
            .unwrap();
        assert_eq!(candidate.innovation(), &vector![2.0, -1.0]);
        assert_relative_eq!(
            candidate.squared_mahalanobis_distance(),
            19.0 / 8.0,
            epsilon = 1e-6
        );
        let expected_log_likelihood =
            -0.5 * (2.0 * (2.0 * std::f32::consts::PI).ln() + 8.0_f32.ln() + 19.0 / 8.0);
        assert_relative_eq!(
            candidate.log_likelihood(),
            expected_log_likelihood,
            epsilon = 1e-6
        );
        assert!(candidate.squared_mahalanobis_distance() < 3.0);
        assert!(distant.squared_mahalanobis_distance() > 3.0);
        assert!(distant.log_likelihood() < candidate.log_likelihood());
        let posterior = candidate.condition().unwrap();
        assert_relative_eq!(
            posterior.mean,
            vector![9.0 / 8.0, -3.0 / 8.0],
            epsilon = 1e-6
        );
        assert_relative_eq!(
            posterior.covariance,
            Matrix2::new(5.0 / 8.0, 1.0 / 8.0, 1.0 / 8.0, 5.0 / 8.0),
            epsilon = 1e-6
        );
        assert_eq!(prior, original);
        // The other evaluation remains usable after conditioning the first.
        assert!(distant.condition().is_ok());
        assert_eq!(prior, original);
    }

    #[test]
    fn likelihood_stays_normalized_for_extreme_finite_covariance_scales() {
        for variance in [1e-30, 1e30] {
            let prior = MultivariateNormalDistribution {
                mean: Vector1::zeros(),
                covariance: Matrix1::new(variance),
            };
            let evaluation = prior
                .evaluate(Matrix1::identity(), Vector1::zeros(), Matrix1::zeros())
                .unwrap();
            let expected = -0.5 * ((2.0 * std::f32::consts::PI).ln() + variance.ln());
            assert_relative_eq!(evaluation.log_likelihood(), expected, epsilon = 1e-5);
            assert_eq!(evaluation.squared_mahalanobis_distance(), 0.0);
        }
    }

    #[test]
    fn failed_prediction_preserves_prior_for_input_and_arithmetic_errors() {
        let original = MultivariateNormalDistribution {
            mean: vector![2.0],
            covariance: Matrix1::identity(),
        };
        for (transition, expected) in [
            (f32::NAN, KalmanError::NonFiniteInput),
            (f32::MAX, KalmanError::NonFiniteResult),
        ] {
            let mut prior = original;
            assert_eq!(
                prior.predict(
                    Matrix1::new(transition),
                    Matrix1::zeros(),
                    Vector1::zeros(),
                    Matrix1::zeros()
                ),
                Err(expected)
            );
            assert_eq!(prior, original);
        }
    }

    #[test]
    fn failed_updates_preserve_prior_through_evaluation_and_conditioning_errors() {
        let original = MultivariateNormalDistribution {
            mean: vector![0.0],
            covariance: Matrix1::zeros(),
        };
        let mut prior = original;
        assert_eq!(
            prior.update(
                Matrix1::identity(),
                vector![f32::INFINITY],
                Matrix1::identity()
            ),
            Err(KalmanError::NonFiniteInput)
        );
        assert_eq!(prior, original);
        assert_eq!(
            prior.update(Matrix1::identity(), vector![0.0], Matrix1::zeros()),
            Err(KalmanError::InvalidInnovationCovariance)
        );
        assert_eq!(prior, original);
        assert_eq!(
            prior.update(Matrix1::identity(), vector![f32::MAX], Matrix1::identity()),
            Err(KalmanError::NonFiniteResult)
        );
        assert_eq!(prior, original);

        let original = MultivariateNormalDistribution {
            mean: vector![3e38],
            covariance: Matrix1::new(1e38),
        };
        let h = Matrix1::new(1e-38);
        let evaluation = original
            .evaluate(h, vector![4.0], Matrix1::zeros())
            .unwrap();
        assert_eq!(evaluation.condition(), Err(KalmanError::NonFiniteResult));
        let mut prior = original;
        assert_eq!(
            prior.update(h, vector![4.0], Matrix1::zeros()),
            Err(KalmanError::NonFiniteResult)
        );
        assert_eq!(prior, original);
    }

    #[test]
    fn singular_prior_and_noise_are_valid_with_positive_definite_innovation() {
        let mut prior = MultivariateNormalDistribution {
            mean: vector![1.0, 2.0],
            covariance: Matrix2::from_diagonal(&vector![0.0, 2.0]),
        };
        prior
            .update(
                Matrix2::identity(),
                vector![3.0, 4.0],
                Matrix2::from_diagonal(&vector![1.0, 0.0]),
            )
            .unwrap();
        assert_relative_eq!(prior.mean, vector![1.0, 4.0], epsilon = 1e-6);
        assert_relative_eq!(prior.covariance, Matrix2::zeros(), epsilon = 1e-12);
    }

    #[test]
    fn repeated_predictions_and_joseph_updates_remain_numerically_stable() {
        let mut state = MultivariateNormalDistribution {
            mean: vector![0.0, 1.0],
            covariance: Matrix2::identity(),
        };
        let transition = Matrix2::new(1.0, 0.1, 0.0, 1.0);
        let h = SMatrix::<f32, 1, 2>::new(1.0, 0.0);
        for step in 1..=1000 {
            state
                .predict(
                    transition,
                    SMatrix::<f32, 2, 1>::zeros(),
                    Vector1::zeros(),
                    Matrix2::from_diagonal(&vector![1e-4, 1e-5]),
                )
                .unwrap();
            state
                .update(h, vector![step as f32 * 0.1], Matrix1::new(1e-3))
                .unwrap();
            assert_relative_eq!(
                state.covariance,
                state.covariance.transpose(),
                epsilon = 1e-7
            );
            assert!(state.covariance.cholesky().is_some());
        }
        assert_relative_eq!(state.mean, vector![100.0, 1.0], epsilon = 1e-3);
    }
}
