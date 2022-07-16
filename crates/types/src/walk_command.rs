use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use super::{KickVariant, Side, Step};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
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
