use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(
    Default, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, SerializeHierarchy,
)]
pub enum Role {
    DefenderLeft,
    DefenderRight,
    Keeper,
    Loser,
    ReplacementKeeper,
    Searcher,
    #[default]
    Striker,
    StrikerSupporter,
}
