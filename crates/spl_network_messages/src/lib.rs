mod bindings;
mod game_controller_return_message;
mod game_controller_state_message;
mod visual_referee_message;

use std::{
    fmt::{self, Display, Formatter},
    time::Duration,
};

use coordinate_systems::Field;
use linear_algebra::{Point2, Pose2};
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
    pub pose: Pose2<Field>,
    pub over_arms_pose_detected: bool,
    pub ball_position: Option<BallPosition<Field>>,
    pub time_to_reach_kick_position: Option<Duration>,
}

pub struct ReadyVisualRefereeMessage {
    pub over_arms_pose: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct BallPosition<Frame> {
    pub position: Point2<Frame>,
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

    use linear_algebra::{Point, Pose2};

    use crate::{BallPosition, HulkMessage, PlayerNumber};

    #[test]
    fn maximum_hulk_message_size() {
        let test_message = HulkMessage {
            player_number: PlayerNumber::Seven,
            fallen: false,
            pose: Pose2::default(),
            over_arms_pose_detected: false,
            ball_position: Some(BallPosition {
                position: Point::origin(),
                age: Duration::MAX,
            }),
            time_to_reach_kick_position: Some(Duration::MAX),
        };
        assert!(bincode::serialize(&test_message).unwrap().len() <= 128)
    }
}
