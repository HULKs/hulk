#[cfg(feature = "ros_z")]
mod bindings;
#[cfg(feature = "ros_z")]
mod game_controller_return_message;
#[cfg(feature = "ros_z")]
mod game_controller_state_message;

use std::fmt::{self, Display, Formatter};
#[cfg(feature = "ros_z")]
use std::time::Duration;

#[cfg(feature = "ros_z")]
use coordinate_systems::Field;
#[cfg(feature = "ros_z")]
use linear_algebra::{Point2, Pose2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ros_z")]
pub use game_controller_return_message::GameControllerReturnMessage;
#[cfg(feature = "ros_z")]
pub use game_controller_state_message::{
    GameControllerStateMessage, GamePhase, GameState, Half, Penalty, PenaltyShoot, Player,
    SubState, Team, TeamColor, TeamState,
};

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Serialize,
    PathDeserialize,
    PathIntrospect,
    PathSerialize,
)]
#[cfg_attr(feature = "ros_z", derive(ros_z::Message))]
#[cfg(feature = "ros_z")]
pub enum HulkMessage {
    State(StateMessage),
}

#[cfg(feature = "ros_z")]
impl Default for HulkMessage {
    fn default() -> Self {
        HulkMessage::State(StateMessage::default())
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "ros_z", derive(ros_z::Message))]
#[cfg(feature = "ros_z")]
pub struct StrikerMessage {
    pub player_number: PlayerNumber,
    pub pose: Pose2<Field>,
    pub ball_position: BallPosition<Field>,
    pub time_to_reach_kick_position: Duration,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathDeserialize,
    PathIntrospect,
    PathSerialize,
)]
#[cfg_attr(feature = "ros_z", derive(ros_z::Message))]
#[cfg(feature = "ros_z")]
pub struct StateMessage {
    pub player_number: PlayerNumber,
    pub pose: Pose2<Field>,
    pub ball_position: Option<BallPosition<Field>>,
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
#[cfg_attr(feature = "ros_z", derive(ros_z::Message))]
#[cfg(feature = "ros_z")]
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
#[cfg_attr(feature = "ros_z", derive(ros_z::Message))]
pub enum PlayerNumber {
    One,
    Two,
    #[default]
    Three,
    Four,
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

#[cfg(all(test, feature = "ros_z"))]
mod tests {
    use super::*;
    #[test]
    fn hulk_striker_message_size() {
        let test_message = HulkMessage::State(StateMessage {
            player_number: PlayerNumber::Five,
            pose: Pose2::default(),
            ball_position: Some(BallPosition {
                position: Point2::origin(),
                age: Duration::MAX,
            }),
        });
        assert!(bincode::serialize(&test_message).unwrap().len() <= 128)
    }
}
