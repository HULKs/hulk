use std::ops::{Add, Sub};

use nalgebra::Vector2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    Default,
)]
pub struct Step {
    pub forward: f32,
    pub left: f32,
    pub turn: f32,
}

impl Step {
    pub const ZERO: Self = Self {
        forward: 0.0,
        left: 0.0,
        turn: 0.0,
    };

    pub fn mirrored(self) -> Self {
        Self {
            forward: self.forward,
            left: -self.left,
            turn: -self.turn,
        }
    }

    pub fn offsets(self) -> Vector2<f32> {
        Vector2::new(self.forward, self.left)
    }

    /// Element wise division, with 0.0 as the result if the divisor is 0.0
    pub fn div_or_zero(self, rhs: &Self) -> Self {
        Self {
            forward: if rhs.forward == 0.0 {
                0.0
            } else {
                self.forward / rhs.forward
            },
            left: if rhs.left == 0.0 {
                0.0
            } else {
                self.left / rhs.left
            },
            turn: if rhs.turn == 0.0 {
                0.0
            } else {
                self.turn / rhs.turn
            },
        }
    }
}

impl Add for Step {
    type Output = Step;

    fn add(self, right: Step) -> Self::Output {
        Self {
            forward: self.forward + right.forward,
            left: self.left + right.left,
            turn: self.turn + right.turn,
        }
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
