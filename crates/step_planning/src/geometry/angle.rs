use std::ops::{Add, Div, Mul, Neg, Sub};

use approx::AbsDiffEq;
use nalgebra::RealField;
use num_traits::Euclid;

#[derive(Clone, Copy, Debug)]
pub struct Angle<T>(pub T);

impl<T> Angle<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: Euclid + RealField> PartialEq for Angle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.absolute_difference(other.clone()) == T::zero()
    }
}

impl<T: Euclid + RealField + Clone> AbsDiffEq for Angle<T> {
    type Epsilon = T::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        T::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.absolute_difference(other.clone())
            .abs_diff_eq(&T::zero(), epsilon)
    }
}

impl<T: RealField + Euclid> Angle<T> {
    pub fn normalized(&self) -> Self {
        Self(self.0.rem_euclid(&T::two_pi()))
    }

    pub fn absolute_difference(&self, to: Self) -> T {
        let counterclockwise_difference = (to - self.clone()).normalized().0;
        if counterclockwise_difference > T::pi() {
            T::two_pi() - counterclockwise_difference
        } else {
            counterclockwise_difference
        }
    }

    pub fn pi() -> Self {
        Self(T::pi())
    }

    pub fn two_pi() -> Self {
        Self(T::two_pi())
    }
}

impl<T: Add<Output = T>> Add for Angle<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl<T: Sub<Output = T>> Sub for Angle<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Angle(self.0 - rhs.0)
    }
}

impl<T: Mul<Output = T>> Mul<T> for Angle<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl<T: Div<Output = T>> Div for Angle<T> {
    type Output = T;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl<T: Div<Output = T>> Div<T> for Angle<T> {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Angle(self.0 / rhs)
    }
}

impl<T: Neg<Output = T>> Neg for Angle<T> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Angle(-self.0)
    }
}
