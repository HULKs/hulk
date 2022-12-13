use serde::{Deserialize, Serialize};
use spl_network_messages::Team;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum FilteredGameState {
    #[default]
    Initial,
    Ready {
        kicking_team: Team,
    },
    Set,
    Playing {
        ball_is_free: bool,
    },
    Finished,
}
