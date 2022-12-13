use std::time::SystemTime;

use filtering::KalmanFilter;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallFilterHypothesis {
    pub filter: KalmanFilter<4>,
    pub validity: f32,
    pub last_update: SystemTime,
}
