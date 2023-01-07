use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use spl_network_messages::Team;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub enum FilteredGameState {
    Initial,
    Ready { kicking_team: Team },
    Set,
    Playing { ball_is_free: bool },
    Finished,
}
