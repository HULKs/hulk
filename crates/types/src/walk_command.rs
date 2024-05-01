use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::{motion_command::KickVariant, step_plan::Step, support_foot::Side};

#[derive(
    Default,
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
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
