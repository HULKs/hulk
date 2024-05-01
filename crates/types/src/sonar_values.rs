use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Default, Debug, Clone, PathSerialize, PathDeserialize, PathIntrospect, Serialize, Deserialize,
)]
pub struct SonarValues {
    pub left_sonar: bool,
    pub right_sonar: bool,
    pub filtered_left_sonar_value: f32,
    pub filtered_right_sonar_value: f32,
}
