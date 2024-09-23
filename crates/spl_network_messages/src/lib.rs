pub mod bindings;
mod game_controller_return_message;
mod game_controller_state_message;

use std::time::Duration;

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
    VisualReferee(VisualRefereeMessage),
}

impl Default for HulkMessage {
    fn default() -> Self {
        HulkMessage::Striker(StrikerMessage::default())
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct StrikerMessage {
    pub jersey_number: usize,
    pub pose: Pose2<Field>,
    pub ball_position: Option<BallPosition<Field>>,
    pub time_to_reach_kick_position: Option<Duration>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct VisualRefereeMessage {
    pub jersey_number: usize,
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use linear_algebra::{Point, Pose2};

    use crate::{BallPosition, HulkMessage, StrikerMessage, VisualRefereeMessage};

    #[test]
    fn hulk_striker_message_size() {
        let test_message = HulkMessage::Striker(StrikerMessage {
            jersey_number: 7,
            pose: Pose2::default(),
            ball_position: Some(BallPosition {
                position: Point::origin(),
                age: Duration::MAX,
            }),
            time_to_reach_kick_position: Some(Duration::MAX),
        });
        assert!(bincode::serialize(&test_message).unwrap().len() <= 128)
    }

    #[test]
    fn hulk_visual_referee_message_size() {
        let test_message = HulkMessage::VisualReferee(VisualRefereeMessage { jersey_number: 4 });
        assert!(bincode::serialize(&test_message).unwrap().len() <= 128)
    }
}
