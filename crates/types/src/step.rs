use std::ops::{Add, AddAssign, Mul, Sub};

use approx::AbsDiffEq;
use nalgebra::RealField;
use num_traits::Euclid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, ros_z::Message)]
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
