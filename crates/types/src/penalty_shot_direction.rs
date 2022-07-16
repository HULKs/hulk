use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub enum PenaltyShotDirection {
    NotMoving,
    Left,
    Right,
}
