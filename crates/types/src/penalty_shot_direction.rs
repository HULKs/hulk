use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum PenaltyShotDirection {
    NotMoving,
    Center,
    Left,
    Right,
}
