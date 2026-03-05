use std::fmt::Debug;

use crate::{Condition, condition::Response};

use serde::{Deserialize, Serialize};
use types::condition_input::ConditionInput;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoGroundContactAbort {}

impl Condition for NoGroundContactAbort {
    fn evaluate(&self, condition_input: &ConditionInput) -> Response {
        if condition_input.ground_contact {
            Response::Continue
        } else {
            Response::Abort
        }
    }
}
