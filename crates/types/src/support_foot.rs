use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ros_z::Message)]
pub enum SupportFootState {
    Left,
    Right,
    Both,
}
