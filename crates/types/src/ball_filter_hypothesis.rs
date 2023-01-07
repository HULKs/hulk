use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::kalman_filter::KalmanFilterSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallFilterHypothesis {
    pub filter: KalmanFilterSnapshot<4>,
    pub validity: f32,
    pub last_update: SystemTime,
}
