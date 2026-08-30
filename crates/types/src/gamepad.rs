use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use ros_z::Message;
use serde::{Deserialize, Serialize};

use crate::motion_command::MotionCommand;

#[derive(
    Clone,
    Debug,
    Default,
    Deserialize,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    Message,
)]
pub struct GamepadManualMotion {
    pub active: bool,
    pub motion_command: MotionCommand,
}
