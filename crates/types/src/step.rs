use std::ops::{Add, AddAssign, Mul, Sub};

use approx::AbsDiffEq;
use nalgebra::RealField;
use num_traits::Euclid;
use serde::{Deserialize, Serialize};

use linear_algebra::Pose2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

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
pub struct Step<T = f32> {
    pub forward: T,
    pub left: T,
    pub turn: T,
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

    pub fn from_pose<Frame>(pose: Pose2<Frame>) -> Self {
        Step {
            forward: pose.position().x(),
            left: pose.position().y(),
            turn: pose.orientation().angle(),
        }
    }
}

impl<T: Mul<Output = T> + Clone> Mul<T> for Step<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self {
            forward: self.forward * rhs.clone(),
            left: self.left * rhs.clone(),
            turn: self.turn * rhs,
        }
    }
}

impl<T: RealField + Euclid> PartialEq for Step<T> {
    fn eq(&self, other: &Self) -> bool {
        self.forward.eq(&other.forward) && self.left.eq(&other.left) && self.turn.eq(&other.turn)
    }
}

impl<T: AbsDiffEq + RealField + Euclid> AbsDiffEq for Step<T>
where
    T::Epsilon: Copy,
{
    type Epsilon = T::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        T::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        let counterclockwise_turn_difference = (self.turn - other.turn).rem_euclid(&T::two_pi());
        let turn_difference = if counterclockwise_turn_difference > T::pi() {
            T::two_pi() - counterclockwise_turn_difference
        } else {
            counterclockwise_turn_difference
        };

        self.forward.abs_diff_eq(&other.forward, epsilon)
            && self.left.abs_diff_eq(&other.left, epsilon)
            && turn_difference.abs_diff_eq(&T::zero(), epsilon)
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

impl AddAssign for Step {
    fn add_assign(&mut self, right: Step) {
        *self = *self + right;
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
