use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, ros_z::Message,
)]
pub enum PrimaryState {
    #[default]
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
