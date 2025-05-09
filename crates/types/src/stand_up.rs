use std::time::Duration;

use serde::{Deserialize, Serialize};

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum RemainingStandUpDuration {
    Running(Duration),
    #[default]
    NotRunning,
}

impl RemainingStandUpDuration {
    pub fn is_running(&self) -> bool {
        matches!(self, RemainingStandUpDuration::Running(_))
    }
}

impl From<RemainingStandUpDuration> for Option<Duration> {
    fn from(val: RemainingStandUpDuration) -> Self {
        match val {
            RemainingStandUpDuration::Running(duration) => Some(duration),
            RemainingStandUpDuration::NotRunning => None,
        }
    }
}
