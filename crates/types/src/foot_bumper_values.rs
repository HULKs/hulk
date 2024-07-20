use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Default, Debug, Clone, PathSerialize, PathDeserialize, PathIntrospect, Serialize, Deserialize,
)]
pub struct FootBumperValues {
    pub left_foot_bumper_count: i32,
    pub right_foot_bumper_count: i32,
    pub obstacle_detected_on_left: bool,
    pub obstacle_detected_on_right: bool,
    pub obstacle_detected_on_middle: bool,
}
