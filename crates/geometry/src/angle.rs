use std::ops::{Add, Div, Mul, Sub};

use approx::{AbsDiffEq, RelativeEq};
use nalgebra::RealField;
use num_traits::Euclid;
use serde::{Deserialize, Serialize};

use linear_algebra::{vector, Rotation2, Vector2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::direction::Direction;

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Angle<T>(pub T);

impl<T> Angle<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: Copy + RealField> Angle<T> {
    pub fn from_direction<Frame>(value: Vector2<Frame, T>) -> Self {
        Self(value.y().atan2(value.x()))
    }
}

impl<T: Euclid + RealField> RelativeEq for Angle<T> {
    fn default_max_relative() -> Self::Epsilon {
        T::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.angle_between(other.clone())
            .0
            .relative_eq(&T::zero(), epsilon, max_relative)
    }
}

impl<T: Euclid + RealField> PartialEq for Angle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.angle_between(other.clone()).0 == T::zero()
    }
}

impl<T: Euclid + RealField + Clone> AbsDiffEq for Angle<T> {
    type Epsilon = T::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        T::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.angle_between(other.clone())
            .0
            .abs_diff_eq(&T::zero(), epsilon)
    }
}

impl<T: RealField + Euclid> Angle<T> {
    pub fn cos(&self) -> T {
        self.0.clone().cos()
    }

    pub fn sin(&self) -> T {
        self.0.clone().sin()
    }

    pub fn angle_to(&self, to: Self, direction: Direction) -> Self {
        ((to - self.clone()) * direction.angle_sign::<T>()).normalized()
    }

    pub fn angle_between(&self, to: Self) -> Self {
        let counterclockwise_difference = (to - self.clone()).normalized();
        if counterclockwise_difference.0 > T::pi() {
            Self::two_pi() - counterclockwise_difference
        } else {
            counterclockwise_difference
        }
    }

    pub fn as_direction<Frame>(&self) -> Vector2<Frame, T> {
        vector![self.cos(), self.sin()]
    }

    pub fn normalized(&self) -> Self {
        Angle(self.0.rem_euclid(&T::two_pi()))
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
        Self(self.0 - rhs.0)
    }
}

impl<T: Mul<Output = T>> Mul<T> for Angle<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl<T: RealField, Frame> Mul<Vector2<Frame, T>> for Angle<T> {
    type Output = Vector2<Frame, T>;

    fn mul(self, rhs: Vector2<Frame, T>) -> Self::Output {
        Rotation2::new(self.0) * rhs
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

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_3, FRAC_PI_4, PI};

    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn angle_to() {
        let eps = 1e-15;

        assert_abs_diff_eq!(
            Angle(0.0).angle_to(Angle(FRAC_PI_2), Direction::Clockwise),
            Angle(3.0 * FRAC_PI_2),
            epsilon = eps
        );
        assert_abs_diff_eq!(
            Angle(0.0).angle_to(Angle(FRAC_PI_2), Direction::Counterclockwise),
            Angle(FRAC_PI_2),
            epsilon = eps
        );
        assert_abs_diff_eq!(
            Angle(5.0 * FRAC_PI_3).angle_to(Angle(FRAC_PI_3), Direction::Clockwise),
            Angle(4.0 * FRAC_PI_3),
            epsilon = eps
        );
        assert_abs_diff_eq!(
            Angle(5.0 * FRAC_PI_3).angle_to(Angle(FRAC_PI_3), Direction::Counterclockwise),
            Angle(2.0 * FRAC_PI_3),
            epsilon = eps
        );
    }

    #[test]
    fn angle_between() {
        let eps = 1e-15;

        assert_abs_diff_eq!(
            Angle(0.0).angle_between(Angle(FRAC_PI_2)),
            Angle(FRAC_PI_2),
            epsilon = eps
        );
        assert_abs_diff_eq!(
            Angle(0.0).angle_between(Angle(3.0 * FRAC_PI_2)),
            Angle(FRAC_PI_2),
            epsilon = eps
        );
        assert_abs_diff_eq!(
            Angle(FRAC_PI_4).angle_between(Angle(-FRAC_PI_4)),
            Angle(FRAC_PI_2),
            epsilon = eps
        );
        assert_abs_diff_eq!(
            Angle(PI * 2.0).angle_between(Angle(0.0)),
            Angle(0.0),
            epsilon = eps
        );
    }
}
