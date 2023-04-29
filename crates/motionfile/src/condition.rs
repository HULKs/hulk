use std::{fmt::Debug, time::Duration};

use crate::{FallenAbort, StabilizedCondition};

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use types::ConditionInput;

pub enum Response {
    Abort,
    Continue,
    Wait,
}

#[enum_dispatch(ConditionType)]
pub trait Condition: Clone {
    fn evaluate(&self, condition_input: &ConditionInput, time_since_start: Duration) -> Response;
}

#[enum_dispatch]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    StabilizedCondition,
    FallenAbort,
}
