use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::kalman_filter::KalmanFilterSnapshot;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BallFilterHypothesisSnapshot {
    pub filter: KalmanFilterSnapshot<4>,
    pub validity: f32,
    pub last_update: SystemTime,
}

impl Default for BallFilterHypothesisSnapshot {
    fn default() -> Self {
        Self {
            filter: Default::default(),
            validity: Default::default(),
            last_update: UNIX_EPOCH,
        }
    }
}
