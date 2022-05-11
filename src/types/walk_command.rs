use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

use super::Step;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum WalkAction {
    #[allow(dead_code)]
    Stand,
    Walk,
    Reset,
}

impl Default for WalkAction {
    fn default() -> Self {
        WalkAction::Reset
    }
}

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy, Deserialize)]
pub struct WalkCommand {
    pub step: Step,
    #[leaf]
    pub action: WalkAction,
}
