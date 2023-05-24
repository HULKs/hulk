use std::ops::{Mul, Sub};

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

    pub fn sum(&self) -> f32 {
        self.forward + self.left + self.turn
    }
}

impl Sub<Step> for Step {
    type Output = Step;

    fn sub(self, right: Step) -> Self::Output {
        Self {
            forward: self.forward - right.forward,
            left: self.left - right.left,
            turn: self.turn - right.turn,
        }
    }
}

impl Mul<Step> for Step {
    type Output = Step;

    fn mul(self, rhs: Step) -> Self::Output {
        Step {
            forward: self.forward * rhs.forward,
            left: self.left * rhs.left,
            turn: self.turn * rhs.turn,
        }
    }
}

impl Mul<f32> for Step {
    type Output = Step;

    fn mul(self, rhs: f32) -> Self::Output {
        Step {
            forward: self.forward * rhs,
            left: self.left * rhs,
            turn: self.turn * rhs,
        }
    }
}
