use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{motion_command::KickVariant, step_plan::Step, support_foot::Side};

#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub enum WalkCommand {
    #[default]
    Stand,
    Walk {
        step: Step,
    },
    Kick {
        variant: KickVariant,
        side: Side,
        strength: f32,
    },
}
