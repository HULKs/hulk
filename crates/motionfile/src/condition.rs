use std::{fmt::Debug, time::Duration};

use crate::{FallenAbort, StabilizedCondition};

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use types::condition_input::ConditionInput;

pub enum Response {
    Abort,
    Continue,
    Wait,
}

impl Response {
    pub fn with_timeout(self, timeout: bool) -> Response {
        if timeout {
            Response::Abort
        } else {
            self
        }
    }
}

#[enum_dispatch]
pub trait Condition {
    fn evaluate(&self, condition_input: &ConditionInput) -> Response;
}

#[enum_dispatch]
pub trait TimeOut {
    fn timeout(&self, time_since_start: Duration) -> bool;
}

#[enum_dispatch(Condition, TimeOut)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscreteConditionType {
    StabilizedCondition,
}

#[enum_dispatch(Condition)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContinuousConditionType {
    FallenAbort,
}
