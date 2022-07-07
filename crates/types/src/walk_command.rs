use serde::{Deserialize, Serialize};

use super::{KickVariant, Side, Step};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum WalkCommand {
    Stand,
    Walk(Step),
    Kick(KickVariant, Side),
}

impl Default for WalkCommand {
    fn default() -> Self {
        WalkCommand::Stand
    }
}
