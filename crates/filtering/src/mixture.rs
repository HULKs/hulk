//! Gaussian mixture approximation and Bayesian probability updates.

use nalgebra::{SMatrix, SVector};
use thiserror::Error;

use crate::multivariate_normal_distribution::MultivariateNormalDistribution as Gaussian;

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum MixtureError {
    #[error("weights must be finite and nonnegative")]
    InvalidWeight,
    #[error("weights have no positive mass")]
    NoPositiveMass,
    #[error("active Gaussian contains non-finite values")]
    NonFiniteGaussian,
    #[error("active log likelihood must be finite or negative infinity")]
    InvalidLogLikelihood,
    #[error("all active hypotheses have impossible evidence")]
    ImpossibleMeasurement,
    #[error("non-finite mixture result")]
    NonFiniteResult,
}

/// Bayesian probabilities after incorporating evidence, together with the
/// combined likelihood of that evidence under the normalized prior weights.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reweighting<const K: usize> {
    pub probabilities: [f64; K],
    /// Natural logarithm of the combined likelihood.
    pub log_likelihood: f64,
}

fn maximum_weight(mut weights: impl Iterator<Item = f64>) -> Result<f64, MixtureError> {
    let maximum = weights.try_fold(0.0_f64, |maximum, weight| {
        if !weight.is_finite() || weight < 0.0 {
            Err(MixtureError::InvalidWeight)
        } else {
            Ok(maximum.max(weight))
        }
    })?;
    if maximum == 0.0 {
        return Err(MixtureError::NoPositiveMass);
    }
    Ok(maximum)
}

/// Approximate weighted alternative estimates with one Gaussian, preserving
/// their combined mean and covariance.
///
/// The covariance includes both component covariances and the spread between
/// component means.
///
/// For normalized weights αᵢ, component means μᵢ and covariances Σᵢ:
///
/// ```text
/// μ = ∑ᵢ αᵢ μᵢ
/// Σ = ∑ᵢ αᵢ [Σᵢ + (μᵢ - μ)(μᵢ - μ)ᵀ]
/// ```
///
/// Weights are normalized internally; zero-weight components are ignored.
/// Weights must be finite and nonnegative, with at least one positive weight.
pub fn approximate_mixture<const N: usize>(
    components: &[(f64, &Gaussian<N>)],
) -> Result<Gaussian<N>, MixtureError> {
    let maximum_weight = maximum_weight(components.iter().map(|(weight, _)| *weight))?;
    let scaled_weight_sum: f64 = components
        .iter()
        .map(|(weight, _)| weight / maximum_weight)
        .sum();
    // Compute between-component spread using offsets from a reference mean to
    // avoid artificial variance from rounding a large absolute mixture mean.
    // Use a maximum-weight component: subtracting a distant, low-weight outlier's
    // mean could erase small differences between the dominant component means.
    let reference = components
        .iter()
        .find(|(weight, _)| *weight == maximum_weight)
        .ok_or(MixtureError::NoPositiveMass)?
        .1
        .mean
        .cast::<f64>();
    let mut offset_mean = SVector::<f64, N>::zeros();
    let mut mean_sum = SVector::<f64, N>::zeros();
    let mut mean_correction = SVector::<f64, N>::zeros();
    for &(weight, gaussian) in components {
        if weight == 0.0 {
            continue;
        }
        if gaussian
            .mean
            .iter()
            .chain(gaussian.covariance.iter())
            .any(|value| !value.is_finite())
        {
            return Err(MixtureError::NonFiniteGaussian);
        }
        let scaled_weight = weight / maximum_weight;
        let component_mean = gaussian.mean.cast::<f64>();
        offset_mean += (component_mean - reference) * (scaled_weight / scaled_weight_sum);
        // Sum weighted means directly, using Neumaier's separate rounding correction
        // to preserve small contributions when large positive and negative terms cancel.
        // Centered offsets help compute covariance, but subtracting a large reference
        // can erase those small contributions; adding it back cannot recover them.
        let weighted_mean = component_mean * scaled_weight;
        for ((sum, correction), value) in mean_sum
            .iter_mut()
            .zip(mean_correction.iter_mut())
            .zip(weighted_mean.iter())
        {
            let next_sum = *sum + value;
            *correction += if sum.abs() >= value.abs() {
                (*sum - next_sum) + value
            } else {
                (value - next_sum) + *sum
            };
            *sum = next_sum;
        }
    }
    let covariance = components
        .iter()
        .filter(|(weight, _)| *weight > 0.0)
        .map(|(weight, gaussian)| {
            let difference = (gaussian.mean.cast::<f64>() - reference) - offset_mean;
            (gaussian.covariance.cast::<f64>() + difference * difference.transpose())
                * ((weight / maximum_weight) / scaled_weight_sum)
        })
        .sum::<SMatrix<f64, N, N>>();
    let mean = ((mean_sum + mean_correction) / scaled_weight_sum).cast::<f32>();
    let covariance = covariance.cast::<f32>();
    if mean
        .iter()
        .chain(covariance.iter())
        .any(|value| !value.is_finite())
    {
        return Err(MixtureError::NonFiniteResult);
    }
    Ok(Gaussian { mean, covariance })
}

/// Update hypothesis probabilities from evidence log likelihoods.
///
/// Returns the posterior probabilities and the combined log likelihood of the
/// evidence. For prior weights wᵢ and likelihoods ℓᵢ:
///
/// ```text
/// pᵢ = wᵢ ℓᵢ / (∑ⱼ wⱼ ℓⱼ)
/// log ℓ = log((∑ᵢ wᵢ ℓᵢ) / (∑ᵢ wᵢ))
/// ```
///
/// Weights must be finite, nonnegative and not all zero, but need not sum to one.
/// Zero-weight hypotheses are ignored. A log likelihood of -∞ means impossible
/// evidence; an error is returned if no positive-weight hypothesis can explain it.
pub fn reweight<const K: usize>(
    weights: [f64; K],
    log_likelihoods: [f64; K],
) -> Result<Reweighting<K>, MixtureError> {
    let maximum_weight = maximum_weight(weights.into_iter())?;
    let scaled_weight_sum: f64 = weights.iter().map(|weight| weight / maximum_weight).sum();
    let log_total_weight = maximum_weight.ln() + scaled_weight_sum.ln();
    let likelihood_maximum = weights.iter().zip(log_likelihoods.iter()).try_fold(
        f64::NEG_INFINITY,
        |maximum, (&weight, &likelihood)| {
            if weight == 0.0 {
                Ok(maximum)
            } else if likelihood.is_nan() || likelihood == f64::INFINITY {
                Err(MixtureError::InvalidLogLikelihood)
            } else {
                Ok(maximum.max(likelihood))
            }
        },
    )?;
    if likelihood_maximum == f64::NEG_INFINITY {
        return Err(MixtureError::ImpossibleMeasurement);
    }
    // Center likelihoods before adding log weights, so even a very large
    // common likelihood offset cannot erase the prior probability ratios.
    let log_joint_weights = std::array::from_fn::<_, K, _>(|index| {
        if weights[index] == 0.0 {
            f64::NEG_INFINITY
        } else {
            weights[index].ln() - log_total_weight + (log_likelihoods[index] - likelihood_maximum)
        }
    });
    let maximum_log_joint_weight = log_joint_weights
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let scaled_joint_weights =
        log_joint_weights.map(|value| (value - maximum_log_joint_weight).exp());
    let scaled_joint_weight_sum: f64 = scaled_joint_weights.iter().sum();
    let log_likelihood =
        likelihood_maximum + (maximum_log_joint_weight + scaled_joint_weight_sum.ln());
    if !log_likelihood.is_finite() {
        return Err(MixtureError::NonFiniteResult);
    }
    Ok(Reweighting {
        probabilities: scaled_joint_weights.map(|value| value / scaled_joint_weight_sum),
        log_likelihood,
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use nalgebra::{Matrix2, Vector2};

    use super::*;

    #[test]
    fn approximation_combines_covariances_and_mean_spread_and_ignores_zero_weights() {
        let left = Gaussian {
            mean: Vector2::new(0.0, 1.0),
            covariance: Matrix2::identity(),
        };
        let right = Gaussian {
            mean: Vector2::new(4.0, 3.0),
            covariance: Matrix2::identity() * 3.0,
        };
        let inactive = Gaussian {
            mean: Vector2::repeat(f32::NAN),
            covariance: Matrix2::repeat(f32::NAN),
        };
        let result =
            approximate_mixture(&[(f64::MAX, &left), (f64::MAX, &right), (0.0, &inactive)])
                .unwrap();
        assert_eq!(result.mean, Vector2::new(2.0, 2.0));
        assert_eq!(result.covariance, Matrix2::new(6.0, 2.0, 2.0, 3.0));
        assert_eq!(
            approximate_mixture(&[(0.0, &left)]),
            Err(MixtureError::NoPositiveMass)
        );
        assert_eq!(
            approximate_mixture(&[(1.0, &inactive)]),
            Err(MixtureError::NonFiniteGaussian)
        );
        let extreme = Gaussian {
            mean: Vector2::repeat(f32::MAX),
            ..left
        };
        assert_eq!(
            approximate_mixture(&[(1.0, &left), (1.0, &extreme)]),
            Err(MixtureError::NonFiniteResult)
        );
    }

    #[test]
    fn approximation_preserves_offsets_outliers_and_cancellation_residuals() {
        let large_identical = Gaussian {
            mean: Vector2::repeat(3e38),
            covariance: Matrix2::identity(),
        };
        assert_eq!(
            approximate_mixture(&[(1.0, &large_identical), (2.0, &large_identical)]).unwrap(),
            large_identical
        );
        let dominant = Gaussian {
            mean: Vector2::repeat(1.0),
            covariance: Matrix2::identity(),
        };
        let outlier = Gaussian {
            mean: Vector2::repeat(1e20),
            covariance: Matrix2::identity(),
        };
        for components in [
            [(1e-30, &outlier), (1.0, &dominant)],
            [(1.0, &dominant), (1e-30, &outlier)],
        ] {
            let result = approximate_mixture(&components).unwrap();
            assert_eq!(result.mean, dominant.mean);
            assert_relative_eq!(
                result.covariance,
                Matrix2::repeat(1e10),
                max_relative = 1e-6
            );
        }
        let positive = Gaussian {
            mean: Vector2::repeat(1e18),
            covariance: Matrix2::identity(),
        };
        let negative = Gaussian {
            mean: -positive.mean,
            covariance: positive.covariance,
        };
        for components in [
            [&positive, &negative, &dominant],
            [&positive, &dominant, &negative],
            [&negative, &positive, &dominant],
            [&negative, &dominant, &positive],
            [&dominant, &positive, &negative],
            [&dominant, &negative, &positive],
        ] {
            let result =
                approximate_mixture(&components.map(|component| (1.0, component))).unwrap();
            assert_eq!(result.mean, Vector2::repeat(1.0 / 3.0));
            assert_relative_eq!(
                result.covariance,
                Matrix2::repeat(2e36 / 3.0),
                max_relative = 1e-6
            );
        }
    }

    #[test]
    fn likelihood_is_stable_and_preserves_tiny_original_weights() {
        let result = reweight([1.0, 3.0], [-1000.0, -1002.0]).unwrap();
        assert_relative_eq!(
            result.log_likelihood,
            -1000.0 + (0.25 + 0.75 * (-2.0_f64).exp()).ln(),
            epsilon = 1e-12
        );
        assert_relative_eq!(
            result.probabilities[0],
            1.0 / (1.0 + 3.0 * (-2.0_f64).exp()),
            epsilon = 1e-12
        );
        let common_offset = reweight([1.0, 3.0], [-1e300, -1e300]).unwrap();
        assert_relative_eq!(common_offset.probabilities[0], 0.25, epsilon = 1e-15);
        let tiny = reweight([1e-300, 1e300], [1400.0, 0.0]).unwrap();
        assert!(tiny.probabilities[0] > 0.999);
        assert!(tiny.log_likelihood > 18.0);
        assert_eq!(
            reweight([1.0, 0.0], [0.0, f64::NAN]).unwrap().probabilities,
            [1.0, 0.0]
        );
        assert_eq!(
            reweight([1.0, 1.0], [0.0, f64::NEG_INFINITY])
                .unwrap()
                .probabilities,
            [1.0, 0.0]
        );
        assert_eq!(
            reweight([1.0, 1.0], [f64::NEG_INFINITY; 2]),
            Err(MixtureError::ImpossibleMeasurement)
        );
        for invalid in [f64::NAN, f64::INFINITY] {
            assert_eq!(
                reweight([1.0], [invalid]),
                Err(MixtureError::InvalidLogLikelihood)
            );
        }
        for invalid in [-1.0, f64::INFINITY, f64::NAN] {
            assert_eq!(reweight([invalid], [0.0]), Err(MixtureError::InvalidWeight));
        }
        assert_eq!(reweight([0.0], [0.0]), Err(MixtureError::NoPositiveMass));
    }
}
