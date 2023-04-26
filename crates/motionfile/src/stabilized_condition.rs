use std::{fmt::Debug, time::Duration};

use crate::Condition;

use serde::{Deserialize, Serialize};
use types::ConditionInput;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilizedCondition {
    tolerance: f32,
    timeout_duration: Duration,
}

impl Condition for StabilizedCondition {
    fn is_fulfilled(&self, condition_input: &ConditionInput, time_since_start: Duration) -> bool {
        condition_input.filtered_angular_velocity.norm() < self.tolerance
            || time_since_start > self.timeout_duration
    }
}
