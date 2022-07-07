use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PrimaryState {
    Unstiff,
    Initial,
    Ready,
    Set,
    Playing,
    Penalized,
    Finished,
    Calibration,
}

impl Default for PrimaryState {
    fn default() -> Self {
        PrimaryState::Unstiff
    }
}
