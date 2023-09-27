use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{motion_command::KickVariant, step_plan::Step, support_foot::Side};

pub type Strength = f32;

#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub enum WalkCommand {
    #[default]
    Stand,
    Walk(Step),
    Kick(KickVariant, Side, Strength),
}
