use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct StepAdjustment {
    pub adjustment: f32,
    pub limited_adjustment: f32,
    pub torso_tilt_shift: f32,
    pub forward_balance_limit: f32,
    pub backward_balance_limit: f32,
}
