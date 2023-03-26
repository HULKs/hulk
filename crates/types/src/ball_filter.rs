use std::time::SystemTime;

use filtering::KalmanFilter;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hypothesis {
    pub filter: KalmanFilter<4>,
    pub validity: f32,
    pub last_update: SystemTime,
}
