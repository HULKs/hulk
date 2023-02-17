mod bindings;
mod game_controller_return_message;
mod game_controller_state_message;
mod spl_message;

use std::{
    fmt::{self, Display, Formatter},
    time::Duration,
};

use nalgebra::Point2;
use serde::{Deserialize, Serialize};

pub use game_controller_return_message::GameControllerReturnMessage;
pub use game_controller_state_message::{
    GameControllerStateMessage, GamePhase, GameState, Half, Penalty, PenaltyShoot, Player, SetPlay,
    Team, TeamColor, TeamState,
};
use serialize_hierarchy::SerializeHierarchy;
pub use spl_message::SplMessage;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct BallPosition {
    pub relative_position: Point2<f32>,
    pub age: Duration,
}

pub const HULKS_TEAM_NUMBER: u8 = 24;

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SerializeHierarchy,
)]
pub enum PlayerNumber {
    One,
    Two,
    Three,
    Four,
    #[default]
    Five,
}

impl Display for PlayerNumber {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let number = match self {
            PlayerNumber::One => "1",
            PlayerNumber::Two => "2",
            PlayerNumber::Three => "3",
            PlayerNumber::Four => "4",
            PlayerNumber::Five => "5",
        };

        write!(formatter, "{number}")
    }
}

impl TryFrom<usize> for PlayerNumber {
    type Error = &'static str;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        let number = match value {
            1 => PlayerNumber::One,
            2 => PlayerNumber::Two,
            3 => PlayerNumber::Three,
            4 => PlayerNumber::Four,
            5 => PlayerNumber::Five,
            _ => return Err("invalid player number"),
        };

        Ok(number)
    }
}
