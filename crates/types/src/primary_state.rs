use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SerializeHierarchy,
)]
pub enum PrimaryState {
    #[default]
    Unstiff,
    Initial,
    Ready,
    Set,
    Playing,
    Penalized,
    Finished,
    Calibration,
}
