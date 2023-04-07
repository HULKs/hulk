use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(
    Default, Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, SerializeHierarchy,
)]
pub enum CameraPosition {
    #[default]
    Top,
    Bottom,
}
