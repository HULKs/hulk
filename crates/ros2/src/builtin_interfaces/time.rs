use color_eyre::Result;
use std::time::{Duration, SystemTime};

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

/// This message communicates ROS Time defined here:
/// https://design.ros2.org/articles/clock_and_time.html
#[repr(C)]
#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathIntrospect, PathSerialize, PathDeserialize,
)]
pub struct Time {
    /// The seconds component, valid over all int32 values.
    pub sec: i32,

    /// The nanoseconds component, valid in the range [0, 1e9), to be added to the seconds component.
    /// e.g.
    /// The time -1.7 seconds is represented as {sec: -2, nanosec: 3e8}
    /// The time 1.7 seconds is represented as {sec: 1, nanosec: 7e8}
    pub nanosec: u32,
}

impl From<Time> for SystemTime {
    fn from(time: Time) -> Self {
        let second_duration = Duration::from_secs(time.sec as u64);
        SystemTime::UNIX_EPOCH
            + Duration::from_nanos(time.nanosec as u64 + second_duration.as_nanos() as u64)
    }
}

impl From<SystemTime> for Time {
    fn from(system_time: SystemTime) -> Self {
        let duration = system_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("no time earlier than UNIX_EPOCH");
        Time {
            sec: duration.as_secs() as i32,
            nanosec: duration.subsec_nanos(),
        }
    }
}
