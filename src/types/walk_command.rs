use serde::{Deserialize, Serialize};

use super::Step;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum WalkCommand {
    Stand,
    Walk(Step),
}

impl Default for WalkCommand {
    fn default() -> Self {
        WalkCommand::Stand
    }
}
