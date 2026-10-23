use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct ImuBiasEstimate {
    pub gyroscope: linear_algebra::Vector3<coordinate_systems::Robot, f64>,
    pub accelerometer: linear_algebra::Vector3<coordinate_systems::Robot, f64>,
    /// Interpolated marginal, ordered gyroscope then accelerometer; includes
    /// cross-correlations between the bracketing calibration knots. Rows of a 6×6 matrix.
    pub covariance: [[f64; 6]; 6],
}

/// Diagnostics for one solve attempt. Missing costs were not reported by the solver.
#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct SolveDiagnostics {
    pub time: Time,
    pub epoch: u64,
    pub duration: Duration,
    /// Complete Localization::solve, including discarded recovery candidates.
    #[serde(default)]
    pub estimation_duration: Duration,
    /// Input ingestion/factor construction, populated by the caller.
    #[serde(default)]
    pub ingestion_duration: Duration,
    pub iterations: Option<usize>,
    pub lm_attempts: usize,
    pub lm_rejected_steps: usize,
    pub gradient_norm: Option<f64>,
    /// Replaced a rejected field-conditioned graph with a validated motion-only window.
    pub motion_rebuilt: bool,
    /// From the optimizer report, or its unchanged cost if no step was accepted.
    /// Unavailable when optimization/covariance fails after accepted steps.
    pub initial_cost: Option<f64>,
    pub final_cost: Option<f64>,
    pub termination: String,
    pub state_count: usize,
    pub measurement_count: usize,
    pub failure: Option<String>,
    pub imu_bias: Option<ImuBiasEstimate>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros_z::message::{SerdeCdrCodec, WireDecoder, WireEncoder};

    #[test]
    fn calibration_diagnostics_have_a_schema_and_round_trip() {
        SolveDiagnostics::schema();
        let value = ImuBiasEstimate {
            gyroscope: linear_algebra::Vector3::zeros(),
            accelerometer: linear_algebra::Vector3::wrap(nalgebra::vector![0.01, 0.03, -0.02]),
            covariance: std::array::from_fn(|r| {
                std::array::from_fn(|c| if r == c { 0.04 } else { 0.001 })
            }),
        };
        let bytes = SerdeCdrCodec::<ImuBiasEstimate>::serialize(&value).unwrap();
        let decoded = SerdeCdrCodec::<ImuBiasEstimate>::deserialize(&bytes).unwrap();
        assert_eq!(decoded.accelerometer, value.accelerometer);
        assert_eq!(decoded.covariance, value.covariance);
    }
}
