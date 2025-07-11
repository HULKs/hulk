mod bindings;
mod game_controller_return_message;
mod game_controller_state_message;

use std::{
    fmt::{self, Display, Formatter},
    time::Duration,
};

use coordinate_systems::Field;
use linear_algebra::{Point2, Pose2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

pub use game_controller_return_message::GameControllerReturnMessage;
pub use game_controller_state_message::{
    GameControllerStateMessage, GamePhase, GameState, Half, Penalty, PenaltyShoot, Player,
    SubState, Team, TeamColor, TeamState,
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum HulkMessage {
    Striker(StrikerMessage),
    Loser(LoserMessage),
    VisualReferee(VisualRefereeMessage),
}

impl Default for HulkMessage {
    fn default() -> Self {
        HulkMessage::Striker(StrikerMessage::default())
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct StrikerMessage {
    pub player_number: PlayerNumber,
    pub pose: Pose2<Field>,
    pub ball_position: BallPosition<Field>,
    pub time_to_reach_kick_position: Duration,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct LoserMessage {
    pub player_number: PlayerNumber,
    pub pose: Pose2<Field>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct VisualRefereeMessage {
    pub player_number: PlayerNumber,
    pub kicking_team: Option<Team>,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    PathDeserialize,
    PathIntrospect,
    PathSerialize,
    Serialize,
)]
pub struct BallPosition<Frame> {
    pub position: Point2<Frame>,
    pub age: Duration,
}

pub const HULKS_TEAM_NUMBER: u8 = 24;
pub const NONE_TEAM_NUMBER: u8 = 255;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    PathDeserialize,
    PathIntrospect,
    PathSerialize,
    Serialize,
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
    use super::*;

    use linear_algebra::Point;

    #[test]
    fn hulk_striker_message_size() {
        let test_message = HulkMessage::Striker(StrikerMessage {
            player_number: PlayerNumber::Seven,
            pose: Pose2::default(),
            ball_position: BallPosition {
                position: Point::origin(),
                age: Duration::MAX,
            },
            time_to_reach_kick_position: Duration::MAX,
        });
        assert!(bincode::serialize(&test_message).unwrap().len() <= 128)
    }

    #[test]
    fn hulk_loser_message_size() {
        let test_message = HulkMessage::Loser(LoserMessage {
            player_number: PlayerNumber::Seven,
            pose: Pose2::default(),
        });
        assert!(bincode::serialize(&test_message).unwrap().len() <= 128)
    }

    #[test]
    fn hulk_visual_referee_message_size() {
        let test_message = HulkMessage::VisualReferee(VisualRefereeMessage {
            player_number: PlayerNumber::Four,
            kicking_team: Some(Team::Hulks),
        });
        assert!(bincode::serialize(&test_message).unwrap().len() <= 128)
    }
}
