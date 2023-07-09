use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Default, Debug, Clone, SerializeHierarchy, Serialize, Deserialize)]
pub struct FootBumperValues {
    pub left_foot_bumper_count: i32,
    pub right_foot_bumper_count: i32,
    pub obstacle_deteced_on_left: bool,
    pub obstacle_deteced_on_right: bool,
}
