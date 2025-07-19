use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Default,
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum Role {
    DefenderLeft,
    DefenderRight,
    Keeper,
    Loser,
    MidfielderLeft,
    MidfielderRight,
    ReplacementKeeper,
    Searcher,
    #[default]
    Striker,
    StrikerSupporter,
}
