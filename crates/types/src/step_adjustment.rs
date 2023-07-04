use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct StepAdjustment {
    pub adjusted_swing_foot: f32,
    pub torso_tilt_shift: f32,
    pub forward_balance_limit: f32,
    pub backward_balance_limit: f32,
    pub adjusted_left_foot_lift: f32,
    pub adjusted_right_foot_lift: f32,
}
