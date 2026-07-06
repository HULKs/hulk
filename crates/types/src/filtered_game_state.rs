use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Message)]
pub enum FilteredGameState {
    #[default]
    Initial,
    Ready,
    Set,
    Playing {
        ball_is_free: bool,
        kick_off: bool,
    },
    Finished,
    Stop,
}
