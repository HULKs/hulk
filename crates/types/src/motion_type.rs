use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum MotionType {
    #[default]
    Prepare,
    Stand,
    StandUp,
    Kick,
    Walk,
}

impl MotionType {
    pub fn is_stable(&self) -> bool {
        matches!(self, MotionType::Stand | MotionType::Prepare)
    }
}
