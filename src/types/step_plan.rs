use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy, Default)]
pub struct Step {
    pub forward: f32,
    pub left: f32,
    pub turn: f32,
}

impl Step {
    pub fn zero() -> Step {
        Step {
            forward: 0.0,
            left: 0.0,
            turn: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy, Deserialize)]
pub struct StepPlan {
    pub step: Step,
}
