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
    Defender,
    Keeper,
    Loser,
    Midfielder,
    ReplacementKeeper,
    Searcher,
    #[default]
    Striker,
    StrikerSupporter,
}
