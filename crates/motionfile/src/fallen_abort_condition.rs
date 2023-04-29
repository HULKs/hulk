use std::{fmt::Debug, time::Duration};

use crate::{condition::Response, Condition};

use serde::{Deserialize, Serialize};
use types::ConditionInput;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallenAbort {}

fn serialize_float_seconds<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f32(duration.as_secs_f32())
}

fn deserialize_float_seconds<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Duration::from_secs_f32(f32::deserialize(deserializer)?))
}

impl Condition for FallenAbort {
    fn evaluate(&self, condition_input: &ConditionInput, _: Duration) -> Response {
        match condition_input.fall_state {
            types::FallState::Fallen { .. } => Response::Abort,
            _ => Response::Continue,
        }
    }
}
