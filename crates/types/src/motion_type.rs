use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Message)]
pub enum MotionType {
    Unstiff,
    Damping,
    Prepare,
    Stand,
    StandUp,
    Kick,
    Walk,
}
