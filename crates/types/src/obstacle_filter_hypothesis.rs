use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{kalman_filter::KalmanFilterSnapshot, ObstacleKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObstacleFilterHypothesisSnapshot {
    pub filter: KalmanFilterSnapshot<2>,
    pub measurement_count: usize,
    pub last_update: SystemTime,
    pub obstacle_kind: ObstacleKind,
}

impl Default for ObstacleFilterHypothesisSnapshot {
    fn default() -> Self {
        Self {
            filter: Default::default(),
            measurement_count: Default::default(),
            last_update: UNIX_EPOCH,
            obstacle_kind: Default::default(),
        }
    }
}
