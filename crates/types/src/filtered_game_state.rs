use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use spl_network_messages::Team;

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
        kicking_team: Option<Team>,
    },
    Set,
    Playing {
        ball_is_free: bool,
        kick_off: bool,
    },
    Finished,
    Standby,
}
