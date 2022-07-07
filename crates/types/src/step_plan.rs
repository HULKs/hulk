use std::ops::Sub;

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy, Default)]
pub struct Step {
    pub forward: f32,
    pub left: f32,
    pub turn: f32,
}

impl Step {
    pub fn zero() -> Self {
        Self {
            forward: 0.0,
            left: 0.0,
            turn: 0.0,
        }
    }

    pub fn mirrored(&self) -> Self {
        Self {
            forward: self.forward,
            left: -self.left,
            turn: -self.turn,
        }
    }
}

impl Sub<Step> for Step {
    type Output = Step;

    fn sub(self, rhs: Step) -> Self::Output {
        Self {
            forward: self.forward - rhs.forward,
            left: self.left - rhs.left,
            turn: self.turn - rhs.turn,
        }
    }
}
