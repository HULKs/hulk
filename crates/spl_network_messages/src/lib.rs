mod bindings;
mod game_controller_return_message;
mod game_controller_state_message;
mod visual_referee_message;

use std::{
    fmt::{self, Display, Formatter},
    time::Duration,
};

use nalgebra::{Isometry2, Point2};
use serde::{Deserialize, Serialize};

pub use game_controller_return_message::GameControllerReturnMessage;
pub use game_controller_state_message::{
    GameControllerStateMessage, GamePhase, GameState, Half, Penalty, PenaltyShoot, Player,
    SubState, Team, TeamColor, TeamState,
};
use serialize_hierarchy::SerializeHierarchy;
pub use visual_referee_message::{VisualRefereeDecision, VisualRefereeMessage};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct HulkMessage {
    pub player_number: PlayerNumber,
    pub fallen: bool,
    pub robot_to_field: Isometry2<f32>,
    pub ball_position: Option<BallPosition>,
    pub time_to_reach_kick_position: Option<Duration>,
}

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
    Five,
    Six,
    #[default]
    Seven,
}

impl Display for PlayerNumber {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let number = match self {
            PlayerNumber::One => "1",
            PlayerNumber::Two => "2",
            PlayerNumber::Three => "3",
            PlayerNumber::Four => "4",
            PlayerNumber::Five => "5",
            PlayerNumber::Six => "6",
            PlayerNumber::Seven => "7",
        };

        write!(formatter, "{number}")
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use nalgebra::Isometry2;

    use crate::{BallPosition, HulkMessage, PlayerNumber};

    #[test]
    fn maximum_hulk_message_size() {
        let test_message = HulkMessage {
            player_number: PlayerNumber::Seven,
            fallen: false,
            robot_to_field: Isometry2::identity(),
            ball_position: Some(BallPosition {
                relative_position: nalgebra::OPoint::origin(),
                age: Duration::MAX,
            }),
            time_to_reach_kick_position: Some(Duration::MAX),
        };
        assert!(bincode::serialize(&test_message).unwrap().len() <= 128)
    }
}
