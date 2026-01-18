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
