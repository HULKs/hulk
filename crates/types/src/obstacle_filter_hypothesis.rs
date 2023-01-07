use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::{kalman_filter::KalmanFilterSnapshot, ObstacleKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObstacleFilterHypothesis {
    pub filter: KalmanFilterSnapshot<2>,
    pub measurement_count: usize,
    pub last_update: SystemTime,
    pub obstacle_kind: ObstacleKind,
}
