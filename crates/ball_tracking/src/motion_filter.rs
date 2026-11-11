//! Stationary/rolling ball motion estimation in a fixed metric frame.
//!
//! Both modes use `[px, py, vx, vy]` in metres and metres/second. The stationary
//! mode has exactly zero velocity mean, velocity covariance and position/velocity
//! cross-covariance, with zero process noise. All input covariances must be
//! symmetric positive semidefinite; these properties are caller obligations.
//! Singular state covariances are supported when innovation covariance is
//! positive definite.

use std::time::Duration;

use filtering::{
    kalman_filter::{KalmanError, KalmanFilter, MeasurementEvaluation},
    mixture::{MixtureError, Reweighting, approximate_mixture, reweight},
    multivariate_normal_distribution::MultivariateNormalDistribution as Gaussian,
};
use nalgebra::{Matrix2, SMatrix, SVector, Vector2};
use thiserror::Error;

use crate::dynamics;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Stationary,
    Rolling,
}

/// A mode's conditional state estimate and its probability.
/// Constructor input probabilities may be unnormalized weights.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModeEstimate {
    pub state: Gaussian<4>,
    pub probability: f64,
}

/// Caller-supplied physical parameters. Scalars must be finite and nonnegative;
/// the launch covariance must be finite, symmetric and positive semidefinite.
/// These parameter preconditions are not checked by the constructor.
#[derive(Clone, Debug, PartialEq)]
pub struct BallMotionParameters {
    /// Constant stationary-to-rolling rate in s⁻¹.
    pub start_rate: f32,
    /// Constant rolling-to-stationary rate in s⁻¹.
    pub stop_rate: f32,
    /// Linear velocity damping in s⁻¹, an approximation of rolling friction.
    pub drag: f32,
    /// Isotropic white acceleration spectral density q in m²/s³.
    /// Without drag, velocity variance increases by q Δt over elapsed time Δt.
    pub acceleration_spectral_density: f32,
    /// Velocity covariance in m²/s² added to stationary-to-rolling contributions.
    pub launch_covariance: Matrix2<f32>,
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum BallMotionError {
    #[error("{mode:?} state contains non-finite values")]
    NonFiniteState { mode: Mode },
    #[error("stationary velocity mean, covariance and cross-covariance must be exactly zero")]
    InvalidStationaryState,
    #[error("non-finite dynamics result")]
    NonFiniteDynamics,
    #[error("mode probabilities: {0}")]
    Probabilities(#[from] MixtureError),
    #[error("{mode:?} mixture approximation: {source}")]
    Mixture {
        mode: Mode,
        #[source]
        source: MixtureError,
    },
    #[error("{mode:?} Kalman operation: {source}")]
    Kalman {
        mode: Mode,
        #[source]
        source: KalmanError,
    },
}

/// Estimates one ball's position, velocity and stationary/rolling motion mode.
/// Uses an interacting multiple-model (IMM) filter with a Gaussian per mode.
///
/// Both estimates remain stored even at zero probability. Each prediction or
/// update is atomic: on error, both estimates and probabilities are unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct BallMotionFilter {
    parameters: BallMotionParameters,
    stationary: ModeEstimate,
    rolling: ModeEstimate,
}

impl BallMotionFilter {
    /// Normalize finite nonnegative probabilities with at least one positive
    /// weight. Both Gaussian inputs must be finite, including zero-weight ones;
    /// the stationary input must satisfy the exact zero-velocity constraints.
    pub fn new(
        parameters: BallMotionParameters,
        mut stationary: ModeEstimate,
        mut rolling: ModeEstimate,
    ) -> Result<Self, BallMotionError> {
        for (mode, estimate) in [(Mode::Stationary, &stationary), (Mode::Rolling, &rolling)] {
            if estimate
                .state
                .mean
                .iter()
                .chain(estimate.state.covariance.iter())
                .any(|value| !value.is_finite())
            {
                return Err(BallMotionError::NonFiniteState { mode });
            }
        }
        if stationary
            .state
            .mean
            .fixed_rows::<2>(2)
            .iter()
            .any(|value| *value != 0.0)
            || stationary
                .state
                .covariance
                .fixed_rows::<2>(2)
                .iter()
                .any(|value| *value != 0.0)
            || stationary
                .state
                .covariance
                .fixed_columns::<2>(2)
                .iter()
                .any(|value| *value != 0.0)
        {
            return Err(BallMotionError::InvalidStationaryState);
        }
        [stationary.probability, rolling.probability] =
            reweight([stationary.probability, rolling.probability], [0.0; 2])?.probabilities;
        Ok(Self {
            parameters,
            stationary,
            rolling,
        })
    }

    fn estimate(&self, mode: Mode) -> &ModeEstimate {
        match mode {
            Mode::Stationary => &self.stationary,
            Mode::Rolling => &self.rolling,
        }
    }

    pub fn state(&self, mode: Mode) -> &Gaussian<4> {
        &self.estimate(mode).state
    }

    pub fn probability(&self, mode: Mode) -> f64 {
        self.estimate(mode).probability
    }

    /// Compute conditional source weights for one destination event.
    /// Prior probabilities μᵢ and transitions πᵢⱼ give destination mass
    /// cⱼ = ∑ᵢ μᵢ πᵢⱼ and conditional source weights μᵢ πᵢⱼ / cⱼ.
    /// A zero event is handled before normalization; positive factors stay
    /// separate until log-domain accumulation to avoid premature product underflow.
    fn transition_weights(
        &self,
        from_stationary: f64,
        from_rolling: f64,
    ) -> Result<Option<Reweighting<2>>, MixtureError> {
        if (self.stationary.probability == 0.0 || from_stationary == 0.0)
            && (self.rolling.probability == 0.0 || from_rolling == 0.0)
        {
            return Ok(None);
        }
        reweight(
            [self.stationary.probability, self.rolling.probability],
            [from_stationary.ln(), from_rolling.ln()],
        )
        .map(Some)
    }

    /// Apply duration-scaled mode transitions, map and approximate the source
    /// mixtures, then propagate each destination. This ordering approximates
    /// switching within the interval, rather than exactly integrating a switched
    /// continuous-time model. Zero elapsed time is an exact no-op.
    ///
    /// Rolling-to-stationary keeps the position marginal and resets velocity,
    /// rather than conditioning on a zero-velocity observation. Launch covariance
    /// is added only to stationary-to-rolling contributions. A zero predicted
    /// destination probability leaves its stored state unchanged. All source
    /// contributions use the original prior, including sources whose own
    /// destination probability becomes zero.
    pub fn predict(&mut self, dt: Duration) -> Result<(), BallMotionError> {
        if dt.is_zero() {
            return Ok(());
        }
        let transitions =
            dynamics::transitions(self.parameters.start_rate, self.parameters.stop_rate, dt)?;
        let to_stationary = self.transition_weights(
            transitions.stationary_to_stationary,
            transitions.rolling_to_stationary,
        )?;
        let to_rolling = self.transition_weights(
            transitions.stationary_to_rolling,
            transitions.rolling_to_rolling,
        )?;
        // Normalize destination masses in log space, rounding only the final
        // probabilities. Conditional source weights remain available separately.
        let probabilities = reweight(
            [1.0; 2],
            [to_stationary, to_rolling]
                .map(|weights| weights.map_or(f64::NEG_INFINITY, |weights| weights.log_likelihood)),
        )?
        .probabilities;
        let mut prediction = self.clone();
        [
            prediction.stationary.probability,
            prediction.rolling.probability,
        ] = probabilities;
        if prediction.stationary.probability > 0.0 {
            let weights = to_stationary
                .ok_or(MixtureError::NoPositiveMass)?
                .probabilities;
            let mut stopped = self.rolling.state;
            stopped.mean.fixed_rows_mut::<2>(2).fill(0.0);
            stopped.covariance.fixed_rows_mut::<2>(2).fill(0.0);
            stopped.covariance.fixed_columns_mut::<2>(2).fill(0.0);
            prediction.stationary.state = approximate_mixture(&[
                (weights[0], &self.stationary.state),
                (weights[1], &stopped),
            ])
            .map_err(|source| BallMotionError::Mixture {
                mode: Mode::Stationary,
                source,
            })?;
        }
        if prediction.rolling.probability > 0.0 {
            let weights = to_rolling
                .ok_or(MixtureError::NoPositiveMass)?
                .probabilities;
            let mut launched = self.stationary.state;
            // The stationary velocity covariance is exactly zero.
            launched
                .covariance
                .fixed_view_mut::<2, 2>(2, 2)
                .copy_from(&self.parameters.launch_covariance);
            let mut rolling =
                approximate_mixture(&[(weights[0], &launched), (weights[1], &self.rolling.state)])
                    .map_err(|source| BallMotionError::Mixture {
                        mode: Mode::Rolling,
                        source,
                    })?;
            let (transition, noise) = dynamics::rolling(
                self.parameters.drag,
                self.parameters.acceleration_spectral_density,
                dt,
            )?;
            rolling
                .predict(
                    transition,
                    SMatrix::<f32, 4, 0>::zeros(),
                    SVector::<f32, 0>::zeros(),
                    noise,
                )
                .map_err(|source| BallMotionError::Kalman {
                    mode: Mode::Rolling,
                    source,
                })?;
            prediction.rolling.state = rolling;
        }
        *self = prediction;
        Ok(())
    }

    /// Evaluate a position observation in the same fixed frame without changing
    /// the prior. Several candidates may coexist. Zero-prior modes are skipped;
    /// every positive-prior mode is evaluated even if its posterior rounds to zero.
    pub fn evaluate(
        &self,
        position: Vector2<f32>,
        noise: Matrix2<f32>,
    ) -> Result<BallMeasurementEvaluation<'_>, BallMotionError> {
        let projection = SMatrix::<f32, 2, 4>::new(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        let evaluate_mode = |mode| {
            let estimate = self.estimate(mode);
            if estimate.probability == 0.0 {
                Ok(None)
            } else {
                estimate
                    .state
                    .evaluate(projection, position, noise)
                    .map(Some)
                    .map_err(|source| BallMotionError::Kalman { mode, source })
            }
        };
        let stationary = evaluate_mode(Mode::Stationary)?;
        let rolling = evaluate_mode(Mode::Rolling)?;
        let posterior = reweight(
            [self.stationary.probability, self.rolling.probability],
            [&stationary, &rolling].map(|evaluation| {
                evaluation.as_ref().map_or(f64::NEG_INFINITY, |evaluation| {
                    f64::from(evaluation.log_likelihood())
                })
            }),
        )?;
        Ok(BallMeasurementEvaluation {
            prior: self,
            stationary,
            rolling,
            log_likelihood: posterior.log_likelihood,
            probabilities: posterior.probabilities,
        })
    }

    /// Evaluate and condition atomically; failures preserve the whole prior.
    pub fn update(
        &mut self,
        position: Vector2<f32>,
        noise: Matrix2<f32>,
    ) -> Result<(), BallMotionError> {
        *self = self.evaluate(position, noise)?.condition()?;
        Ok(())
    }
}

/// A position observation evaluated against an immutably borrowed filter.
/// Dropping this value rejects the candidate; conditioning consumes it and
/// produces an owned posterior, leaving the prior and other candidates intact.
#[derive(Debug)]
pub struct BallMeasurementEvaluation<'a> {
    prior: &'a BallMotionFilter,
    stationary: Option<MeasurementEvaluation<'a, 4, 2>>,
    rolling: Option<MeasurementEvaluation<'a, 4, 2>>,
    log_likelihood: f64,
    probabilities: [f64; 2],
}

impl BallMeasurementEvaluation<'_> {
    /// Natural logarithm of the two-mode mixture measurement density.
    pub fn log_likelihood(&self) -> f64 {
        self.log_likelihood
    }

    pub fn posterior_probability(&self, mode: Mode) -> f64 {
        match mode {
            Mode::Stationary => self.probabilities[0],
            Mode::Rolling => self.probabilities[1],
        }
    }

    /// Kalman innovations, likelihoods and Mahalanobis distances for gating.
    /// Zero-prior modes have no evaluation.
    pub fn mode_evaluation(&self, mode: Mode) -> Option<&MeasurementEvaluation<'_, 4, 2>> {
        match mode {
            Mode::Stationary => self.stationary.as_ref(),
            Mode::Rolling => self.rolling.as_ref(),
        }
    }

    /// Condition every evaluated mode, including ones whose posterior probability
    /// rounds to zero. Unevaluated zero-prior modes retain their stored states.
    pub fn condition(self) -> Result<BallMotionFilter, BallMotionError> {
        let mut posterior = self.prior.clone();
        if let Some(evaluation) = self.stationary {
            posterior.stationary.state =
                evaluation
                    .condition()
                    .map_err(|source| BallMotionError::Kalman {
                        mode: Mode::Stationary,
                        source,
                    })?;
        }
        if let Some(evaluation) = self.rolling {
            posterior.rolling.state =
                evaluation
                    .condition()
                    .map_err(|source| BallMotionError::Kalman {
                        mode: Mode::Rolling,
                        source,
                    })?;
        }
        [
            posterior.stationary.probability,
            posterior.rolling.probability,
        ] = self.probabilities;
        Ok(posterior)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use nalgebra::{Matrix4, Vector4, matrix};

    use super::*;

    fn parameters() -> BallMotionParameters {
        BallMotionParameters {
            start_rate: 0.0,
            stop_rate: 0.0,
            drag: 0.0,
            acceleration_spectral_density: 0.0,
            launch_covariance: Matrix2::zeros(),
        }
    }

    fn estimate(x: f32, probability: f64) -> ModeEstimate {
        ModeEstimate {
            state: Gaussian {
                mean: Vector4::new(x, 0.0, 0.0, 0.0),
                covariance: Matrix4::from_diagonal(&Vector4::new(1.0, 1.0, 0.0, 0.0)),
            },
            probability,
        }
    }

    fn assert_stationary(state: &Gaussian<4>) {
        assert!(
            state
                .mean
                .fixed_rows::<2>(2)
                .iter()
                .all(|value| *value == 0.0)
        );
        assert!(
            state
                .covariance
                .fixed_rows::<2>(2)
                .iter()
                .all(|value| *value == 0.0)
        );
        assert!(
            state
                .covariance
                .fixed_columns::<2>(2)
                .iter()
                .all(|value| *value == 0.0)
        );
    }

    #[test]
    fn stopping_uses_the_prior_position_marginal_even_when_rolling_mass_becomes_zero() {
        let rolling = ModeEstimate {
            state: Gaussian {
                mean: Vector4::new(2.0, -1.0, 3.0, 0.0),
                covariance: matrix![
                    4.0, 0.0, 3.0, 0.0;
                    0.0, 4.0, 0.0, 3.0;
                    3.0, 0.0, 9.0, 0.0;
                    0.0, 3.0, 0.0, 9.0;
                ],
            },
            probability: 1.0,
        };
        for seconds in [1, 1000] {
            let mut filter = BallMotionFilter::new(
                BallMotionParameters {
                    stop_rate: 1.0,
                    ..parameters()
                },
                estimate(-100.0, 0.0),
                rolling,
            )
            .unwrap();
            filter.predict(Duration::from_secs(seconds)).unwrap();
            let stopped = filter.state(Mode::Stationary);
            assert_eq!(stopped.mean, Vector4::new(2.0, -1.0, 0.0, 0.0));
            assert_eq!(
                stopped.covariance.fixed_view::<2, 2>(0, 0),
                Matrix2::identity() * 4.0
            );
            assert_stationary(stopped);
            if seconds == 1 {
                assert_relative_eq!(
                    filter.probability(Mode::Rolling),
                    (-1.0_f64).exp(),
                    epsilon = 1e-15
                );
            } else {
                assert_eq!(filter.probability(Mode::Rolling), 0.0);
                assert_eq!(filter.state(Mode::Rolling), &rolling.state);
            }
            let evaluation = filter
                .evaluate(Vector2::new(4.0, 1.0), Matrix2::identity())
                .unwrap();
            if seconds == 1000 {
                assert!(evaluation.mode_evaluation(Mode::Rolling).is_none());
            }
            filter = evaluation.condition().unwrap();
            let stopped = filter.state(Mode::Stationary);
            assert_stationary(stopped);
            assert_relative_eq!(stopped.mean[0], 3.6, epsilon = 1e-6);
            assert_relative_eq!(stopped.covariance[(0, 0)], 0.8, epsilon = 1e-6);
            if seconds == 1000 {
                assert_eq!(filter.state(Mode::Rolling), &rolling.state);
            }
        }
    }

    #[test]
    fn launch_uncertainty_is_specific_to_stationary_contributions() {
        let launch = Matrix2::new(4.0, 1.0, 1.0, 9.0);
        let parameters = BallMotionParameters {
            start_rate: std::f32::consts::LN_2,
            launch_covariance: launch,
            ..parameters()
        };
        for [stationary_probability, rolling_probability] in [[1.0, 0.0], [0.0, 1.0], [0.5, 0.5]] {
            let mut rolling = estimate(0.0, rolling_probability);
            rolling.state.covariance[(2, 2)] = 2.0;
            rolling.state.covariance[(3, 3)] = 2.0;
            let stationary = estimate(0.0, stationary_probability);
            let mut filter =
                BallMotionFilter::new(parameters.clone(), stationary, rolling).unwrap();
            filter.predict(Duration::from_secs(1)).unwrap();
            let launch_mass =
                stationary_probability * (1.0 - (-f64::from(parameters.start_rate)).exp());
            let fraction = (launch_mass / (launch_mass + rolling_probability)) as f32;
            let expected_velocity =
                launch * fraction + Matrix2::identity() * (2.0 * (1.0 - fraction));
            let state = filter.state(Mode::Rolling);
            assert_relative_eq!(
                state.covariance.fixed_view::<2, 2>(2, 2).into_owned(),
                expected_velocity,
                epsilon = 1e-6
            );
            assert_relative_eq!(
                state.covariance.fixed_view::<2, 2>(0, 2).into_owned(),
                expected_velocity,
                epsilon = 1e-6
            );
            assert_relative_eq!(
                state.covariance.fixed_view::<2, 2>(0, 0).into_owned(),
                Matrix2::identity() + expected_velocity,
                epsilon = 1e-6
            );
            assert_stationary(filter.state(Mode::Stationary));
            assert_eq!(filter.state(Mode::Stationary), &stationary.state);
        }
    }

    #[test]
    fn stationary_mixing_includes_between_position_spread_and_remains_exact() {
        let mut rolling = estimate(4.0, 0.5);
        rolling.state.mean[2] = 2.0;
        rolling.state.covariance[(2, 2)] = 3.0;
        let mut filter = BallMotionFilter::new(
            BallMotionParameters {
                stop_rate: std::f32::consts::LN_2,
                ..parameters()
            },
            estimate(0.0, 0.5),
            rolling,
        )
        .unwrap();
        filter.predict(Duration::from_secs(1)).unwrap();
        let state = filter.state(Mode::Stationary);
        assert_stationary(state);
        assert_relative_eq!(state.mean[0], 4.0 / 3.0, epsilon = 1e-6);
        assert_relative_eq!(state.covariance[(0, 0)], 1.0 + 32.0 / 9.0, epsilon = 1e-6);
        filter
            .update(Vector2::zeros(), Matrix2::identity())
            .unwrap();
        assert_stationary(filter.state(Mode::Stationary));
    }

    #[test]
    fn candidate_likelihoods_remain_separate_and_zero_weight_posteriors_keep_conditioned_states() {
        let filter = BallMotionFilter::new(
            BallMotionParameters {
                start_rate: 1.0,
                ..parameters()
            },
            estimate(-5.0, 0.5),
            estimate(5.0, 0.5),
        )
        .unwrap();
        let original = filter.clone();
        let left = filter
            .evaluate(Vector2::new(-5.0, 0.0), Matrix2::identity())
            .unwrap();
        let right = filter
            .evaluate(Vector2::new(5.0, 0.0), Matrix2::identity())
            .unwrap();
        let expected = -(4.0 * std::f64::consts::PI).ln() + (0.5 + 0.5 * (-25.0_f64).exp()).ln();
        assert_relative_eq!(left.log_likelihood(), expected, epsilon = 1e-6);
        let collapsed = approximate_mixture(&[
            (0.5, filter.state(Mode::Stationary)),
            (0.5, filter.state(Mode::Rolling)),
        ])
        .unwrap();
        let collapsed_likelihood = collapsed
            .evaluate(
                SMatrix::<f32, 2, 4>::new(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0),
                Vector2::new(-5.0, 0.0),
                Matrix2::identity(),
            )
            .unwrap()
            .log_likelihood();
        assert!((left.log_likelihood() - f64::from(collapsed_likelihood)).abs() > 0.5);
        assert_eq!(
            left.mode_evaluation(Mode::Stationary)
                .unwrap()
                .squared_mahalanobis_distance(),
            0.0
        );
        assert_relative_eq!(
            left.mode_evaluation(Mode::Rolling)
                .unwrap()
                .squared_mahalanobis_distance(),
            50.0,
            epsilon = 1e-5
        );
        assert!(left.posterior_probability(Mode::Stationary) > 0.99);
        assert!(right.posterior_probability(Mode::Rolling) > 0.99);
        assert_stationary(left.condition().unwrap().state(Mode::Stationary));
        assert!(right.condition().unwrap().probability(Mode::Rolling) > 0.99);
        assert_eq!(filter, original);

        let evaluation = filter
            .evaluate(Vector2::new(-500.0, 0.0), Matrix2::identity())
            .unwrap();
        assert_eq!(evaluation.posterior_probability(Mode::Rolling), 0.0);
        assert!(evaluation.mode_evaluation(Mode::Rolling).is_some());
        let mut posterior = evaluation.condition().unwrap();
        assert_relative_eq!(
            posterior.state(Mode::Rolling).mean[0],
            -247.5,
            epsilon = 1e-4
        );
        let retained = *posterior.state(Mode::Rolling);
        posterior
            .update(Vector2::new(-250.0, 0.0), Matrix2::identity())
            .unwrap();
        assert_eq!(posterior.state(Mode::Rolling), &retained);
        // Positive incoming mass rebuilds the rolling estimate from the current
        // stationary source, not its retained zero-weight conditional estimate.
        let incoming = *posterior.state(Mode::Stationary);
        assert_ne!(incoming, retained);
        posterior.predict(Duration::from_secs(1)).unwrap();
        assert!(posterior.probability(Mode::Rolling) > 0.0);
        assert_eq!(posterior.state(Mode::Rolling), &incoming);
    }

    #[test]
    fn constructor_checks_both_states_and_normalizes_without_a_fallback() {
        let mut filter =
            BallMotionFilter::new(parameters(), estimate(0.0, 2.0), estimate(3.0, 1.0)).unwrap();
        assert_relative_eq!(
            filter.probability(Mode::Stationary),
            2.0 / 3.0,
            epsilon = 1e-15
        );
        let original = filter.clone();
        filter.predict(Duration::ZERO).unwrap();
        assert_eq!(filter, original);
        for mode in [Mode::Stationary, Mode::Rolling] {
            let mut stationary = estimate(0.0, 1.0);
            let mut rolling = estimate(0.0, 1.0);
            let invalid = match mode {
                Mode::Stationary => &mut stationary,
                Mode::Rolling => &mut rolling,
            };
            invalid.probability = 0.0;
            invalid.state.mean[0] = f32::NAN;
            assert_eq!(
                BallMotionFilter::new(parameters(), stationary, rolling).unwrap_err(),
                BallMotionError::NonFiniteState { mode }
            );
        }
        let mut moving = estimate(0.0, 0.0);
        moving.state.mean[2] = 1.0;
        assert_eq!(
            BallMotionFilter::new(parameters(), moving, estimate(0.0, 1.0)).unwrap_err(),
            BallMotionError::InvalidStationaryState
        );
        assert_eq!(
            BallMotionFilter::new(parameters(), estimate(0.0, 0.0), estimate(0.0, 0.0))
                .unwrap_err(),
            BallMotionError::Probabilities(MixtureError::NoPositiveMass)
        );
    }

    #[test]
    fn tiny_transition_mass_and_very_unlikely_measurements_do_not_lose_representable_probabilities()
    {
        let mut filter = BallMotionFilter::new(
            BallMotionParameters {
                start_rate: 744.0,
                ..parameters()
            },
            estimate(0.0, 1.0),
            estimate(100.0, 1.0),
        )
        .unwrap();
        filter.predict(Duration::from_secs(1)).unwrap();
        assert_eq!(filter.probability(Mode::Stationary), f64::from_bits(1));
        assert_eq!(filter.state(Mode::Stationary).mean[0], 0.0);

        let mut filter =
            BallMotionFilter::new(parameters(), estimate(0.0, 1.0), estimate(0.0, 3.0)).unwrap();
        let position = Vector2::new(1e10, 0.0);
        let evaluation = filter.evaluate(position, Matrix2::identity()).unwrap();
        assert!(evaluation.log_likelihood() < -1e19);
        assert_relative_eq!(
            evaluation.posterior_probability(Mode::Stationary),
            0.25,
            epsilon = 1e-15
        );
        filter.update(position, Matrix2::identity()).unwrap();
        assert_relative_eq!(filter.probability(Mode::Rolling), 0.75, epsilon = 1e-15);
    }

    #[test]
    fn failures_preserve_both_estimates_and_report_the_mode_and_source() {
        let mut filter = BallMotionFilter::new(
            BallMotionParameters {
                acceleration_spectral_density: f32::MAX,
                ..parameters()
            },
            estimate(0.0, 0.5),
            estimate(0.0, 0.5),
        )
        .unwrap();
        let original = filter.clone();
        assert_eq!(
            filter.predict(Duration::MAX),
            Err(BallMotionError::NonFiniteDynamics)
        );
        assert_eq!(filter, original);
        assert_eq!(
            filter.update(Vector2::repeat(f32::NAN), Matrix2::identity()),
            Err(BallMotionError::Kalman {
                mode: Mode::Stationary,
                source: KalmanError::NonFiniteInput
            })
        );
        assert_eq!(filter, original);
        let mut rolling = estimate(0.0, 0.5);
        rolling.state.covariance.fill(0.0);
        let mut filter = BallMotionFilter::new(parameters(), estimate(0.0, 0.5), rolling).unwrap();
        let original = filter.clone();
        assert_eq!(
            filter.update(Vector2::zeros(), Matrix2::zeros()),
            Err(BallMotionError::Kalman {
                mode: Mode::Rolling,
                source: KalmanError::InvalidInnovationCovariance
            })
        );
        assert_eq!(filter, original);

        // Both evaluations succeed, but the second conditioning overflows.
        let mut stationary = estimate(0.0, 0.5);
        stationary.state.covariance[(0, 0)] = 1e-38;
        let mut rolling = stationary;
        rolling.state.mean[2] = 3e38;
        rolling.state.covariance[(0, 2)] = 1.5;
        rolling.state.covariance[(2, 0)] = 1.5;
        rolling.state.covariance[(2, 2)] = 3e38;
        let mut filter = BallMotionFilter::new(parameters(), stationary, rolling).unwrap();
        let original = filter.clone();
        let position = Vector2::new(0.5, 0.0);
        let noise = Matrix2::zeros();
        let evaluation = filter.evaluate(position, noise).unwrap();
        assert!(evaluation.posterior_probability(Mode::Rolling) > 0.0);
        assert_eq!(
            evaluation.condition().unwrap_err(),
            BallMotionError::Kalman {
                mode: Mode::Rolling,
                source: KalmanError::NonFiniteResult
            }
        );
        assert_eq!(
            filter.update(position, noise),
            Err(BallMotionError::Kalman {
                mode: Mode::Rolling,
                source: KalmanError::NonFiniteResult
            })
        );
        assert_eq!(filter, original);
    }
}
