use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    PartialEq,
    Eq,
)]
pub enum FilteredGameState {
    #[default]
    Initial,
    Ready {
        kicking_team_known: bool,
    },
    Set,
    Playing {
        ball_is_free: bool,
        kick_off: bool,
    },
    Finished,
    Standby,
}
