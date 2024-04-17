use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Debug, Clone, Serialize, Default, Deserialize, SerializeHierarchy)]
pub enum PoseType {
    OverheadArms,
    ArmsBySide,
    #[default]
    NoPose,
}
