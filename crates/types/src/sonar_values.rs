use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Default, Debug, Clone, SerializeHierarchy, Serialize, Deserialize)]
pub struct SonarValues {
    pub left_sonar: bool,
    pub right_sonar: bool,
    pub filtered_left_sonar_value: f32,
    pub filtered_right_sonar_value: f32,
}
