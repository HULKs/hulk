use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, ros_z::Message)]
pub enum PrimaryState {
    Unstiff,
    Damping,
    Prepare,
    Stop,
    Initial,
    Ready,
    Set,
    Playing,
    Penalized,
    Finished,
}
