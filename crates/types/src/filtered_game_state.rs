use serde::{Deserialize, Serialize};
use spl_network_messages::Team;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum FilteredGameState {
    Ready { kicking_team: Team },
    Initial,
    Set,
    Playing { ball_is_free: bool },
    Finished,
}

impl Default for FilteredGameState {
    fn default() -> Self {
        Self::Initial
    }
}
