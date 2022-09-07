use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
