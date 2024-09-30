use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use spl_network_messages::Team;

#[derive(
    Clone, Copy, Debug, Default, Serialize, Deserialize, SerializeHierarchy, PartialEq, Eq,
)]
pub enum FilteredGameState {
    #[default]
    Initial,
    Ready {
        kicking_team: Team,
    },
    Set,
    Playing {
        ball_is_free: bool,
        kick_off: bool,
    },
    Finished,
}
