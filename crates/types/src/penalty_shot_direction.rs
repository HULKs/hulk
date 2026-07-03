use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Debug, Deserialize, Serialize, PartialEq)]
pub enum PenaltyShotDirection {
    #[default]
    NotMoving,
    Center,
    Left,
    Right,
}
