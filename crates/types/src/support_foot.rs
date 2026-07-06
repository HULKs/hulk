use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, ros_z::Message)]
pub enum Side {
    #[default]
    Left,
    Right,
}

impl Side {
    pub fn opposite(&self) -> Side {
        match self {
            Side::Left => Self::Right,
            Side::Right => Self::Left,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct SupportFoot {
    pub support_side: Option<Side>,
    pub changed_this_cycle: bool,
}
