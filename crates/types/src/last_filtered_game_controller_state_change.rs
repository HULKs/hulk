use std::time::SystemTime;

use crate::players::Players;
use path_serde::{PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use spl_network_messages::Penalty;

#[derive(Debug, Clone, Serialize, Deserialize, PathSerialize, PathIntrospect, PartialEq)]

pub struct LastFilteredGameControllerStateChanges {
    pub game_state: SystemTime,
    pub opponent_game_state: SystemTime,
    pub game_phase: SystemTime,
    pub kicking_team: SystemTime,
    pub penalties: Players<Option<SystemTime>>,
    pub opponent_penalties: Players<Option<Penalty>>,
    pub sub_state: Option<SystemTime>,
}

impl Default for LastFilteredGameControllerStateChanges {
    fn default() -> Self {
        Self {
            game_state: SystemTime::UNIX_EPOCH,
            opponent_game_state: SystemTime::UNIX_EPOCH,
            game_phase: SystemTime::UNIX_EPOCH,
            kicking_team: SystemTime::UNIX_EPOCH,
            penalties: Players::default(),
            opponent_penalties: Players::default(),
            sub_state: None,
        }
    }
}
