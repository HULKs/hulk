use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub enum PrimaryState {
    Unstiff,
    Initial,
    Ready,
    Set,
    Playing,
    Penalized,
    Finished,
    Calibration,
}
