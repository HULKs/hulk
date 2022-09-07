use std::time::SystemTime;

use filtering::KalmanFilter;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallFilterHypothesis {
    filter: KalmanFilter<4>,
    validity: f32,
    last_update: SystemTime,
}
