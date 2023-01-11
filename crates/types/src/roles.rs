use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, SerializeHierarchy)]
pub enum Role {
    DefenderLeft,
    DefenderRight,
    Keeper,
    Loser,
    ReplacementKeeper,
    Searcher,
    Striker,
    StrikerSupporter,
}

impl Default for Role {
    fn default() -> Self {
        Role::Striker
    }
}
