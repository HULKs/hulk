use nalgebra::{SMatrix, SVector};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KalmanFilterSnapshot<const STATE_DIMENSION: usize> {
    pub state: SVector<f32, STATE_DIMENSION>,
    pub covariance: SMatrix<f32, STATE_DIMENSION, STATE_DIMENSION>,
}
