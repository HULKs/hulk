use std::{fmt::Debug, time::Duration};

use crate::StabilizedCondition;

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use types::ConditionInput;

#[enum_dispatch(ConditionType)]
pub trait Condition: Clone {
    fn is_fulfilled(&self, condition_input: &ConditionInput, time_since_start: Duration) -> bool;
}

#[enum_dispatch]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    StabilizedCondition,
}
