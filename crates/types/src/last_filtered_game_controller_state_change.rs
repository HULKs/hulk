use crate::{cycle_time::CycleTime, players::Players};
use path_serde::{PathDeserialize, PathSerialize};
use serde::{Deserialize, Serialize};
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    PathSerialize,
    PathDeserialize,
)]
pub struct LastFilteredGameControllerStateChanges {
    pub game_state: CycleTime,
    pub opponent_game_state: CycleTime,
    pub game_phase: CycleTime,
    pub kicking_team: CycleTime,
    pub penalties: Players<Option<CycleTime>>,
    pub sub_state: Option<CycleTime>,
}
