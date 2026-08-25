use std::time::SystemTime;

use crate::players::Players;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]

pub struct LastFilteredGameControllerStateChanges {
    pub game_state: SystemTime,
    pub opponent_game_state: SystemTime,
    pub game_phase: SystemTime,
    pub kicking_team: SystemTime,
    pub penalties: Players<Option<SystemTime>>,
    pub sub_state: Option<SystemTime>,
}
