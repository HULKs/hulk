use std::time::SystemTime;

use filtering::KalmanFilter;
use serde::{Deserialize, Serialize};

use crate::ObstacleKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObstacleFilterHypothesis {
    filter: KalmanFilter<2>,
    measurement_count: usize,
    last_update: SystemTime,
    obstacle_kind: ObstacleKind,
}
