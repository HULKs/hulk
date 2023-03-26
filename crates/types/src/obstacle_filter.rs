use std::time::SystemTime;

use filtering::KalmanFilter;
use serde::{Deserialize, Serialize};

use crate::ObstacleKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub filter: KalmanFilter<2>,
    pub measurement_count: usize,
    pub last_update: SystemTime,
    pub obstacle_kind: ObstacleKind,
}
