use std::{fmt::Debug, time::Duration};

use crate::condition::{Condition, Response, TimeOut};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use types::condition_input::ConditionInput;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilizedCondition {
    tolerance: f32,
    #[serde(
        serialize_with = "serialize_float_seconds",
        deserialize_with = "deserialize_float_seconds"
    )]
    timeout_duration: Duration,
}

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

impl Condition for StabilizedCondition {
    fn evaluate(&self, condition_input: &ConditionInput) -> Response {
        if condition_input.filtered_angular_velocity.norm() < self.tolerance {
            return Response::Continue;
        }
        Response::Wait
    }
}

impl TimeOut for StabilizedCondition {
    fn timeout(&self, time_since_start: Duration) -> bool {
        time_since_start > self.timeout_duration
    }
}
