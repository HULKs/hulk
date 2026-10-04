//! Framed variables and measurement models for continuous-time 3D localization.
//!
//! Callers own measurement queues, solve scheduling, and publication policy.
//! Factors evaluate the trajectory at observation times using their knot dependencies.
//!
//! Lie variables, spline evaluation, and measurement factors support scalar-generic
//! analytical derivatives with fixed-size numerical storage.

use fagra::EvaluationError;
use nalgebra::RealField;

pub mod alignment;
mod covariance;
pub mod factors;
pub mod preintegration;
pub mod spline;
pub mod variables;

pub(crate) fn finite<'a, R: RealField + 'a>(
    mut values: impl Iterator<Item = &'a R>,
) -> Result<(), EvaluationError> {
    if values.all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(EvaluationError::InvalidEvaluation)
    }
}
