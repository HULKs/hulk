use std::ops::{Add, Sub};

use approx::{AbsDiffEq, RelativeEq};
use geometry::direction::Direction;
use linear_algebra::{vector, Vector2};
use nalgebra::RealField;
use num_traits::Euclid;

use super::angle::Angle;

#[derive(Clone, Copy, Debug)]
pub struct Orientation<T>(pub T);

impl<T> Orientation<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: Copy + RealField> Orientation<T> {
    pub fn from_direction<Frame>(value: Vector2<Frame, T>) -> Self {
        Self(value.y().atan2(value.x()))
    }
}

impl<T: Euclid + RealField> RelativeEq for Orientation<T> {
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

impl<T: Euclid + RealField> PartialEq for Orientation<T> {
    fn eq(&self, other: &Self) -> bool {
        self.angle_between(other.clone()).0 == T::zero()
    }
}

impl<T: Euclid + RealField + Clone> AbsDiffEq for Orientation<T> {
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

impl<T: RealField + Euclid> Orientation<T> {
    pub fn cos(&self) -> T {
        self.0.clone().cos()
    }

    pub fn sin(&self) -> T {
        self.0.clone().sin()
    }

    pub fn angle_to(&self, to: Self, direction: Direction) -> Angle<T> {
        ((to - self.clone()) * direction.angle_sign::<T>()).normalized()
    }

    pub fn angle_between(&self, to: Self) -> Angle<T> {
        let counterclockwise_difference = (to - self.clone()).normalized();
        if counterclockwise_difference.0 > T::pi() {
            Angle::<T>::two_pi() - counterclockwise_difference
        } else {
            counterclockwise_difference
        }
    }

    pub fn as_direction<Frame>(&self) -> Vector2<Frame, T> {
        vector![self.cos(), self.sin()]
    }

    pub fn normalized(&self) -> Self {
        Self(self.0.rem_euclid(&T::two_pi()))
    }

    pub fn pi() -> Self {
        Self(T::pi())
    }

    pub fn two_pi() -> Self {
        Self(T::two_pi())
    }
}

impl<T: Add<Output = T>> Add<Angle<T>> for Orientation<T> {
    type Output = Self;

    fn add(self, rhs: Angle<T>) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl<T: Sub<Output = T>> Sub<Angle<T>> for Orientation<T> {
    type Output = Self;

    fn sub(self, rhs: Angle<T>) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl<T: Sub<Output = T>> Sub for Orientation<T> {
    type Output = Angle<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        Angle(self.0 - rhs.0)
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
            Orientation(0.0).angle_to(Orientation(FRAC_PI_2), Direction::Clockwise),
            Angle(3.0 * FRAC_PI_2),
            epsilon = eps
        );
        assert_abs_diff_eq!(
            Orientation(0.0).angle_to(Orientation(FRAC_PI_2), Direction::Counterclockwise),
            Angle(FRAC_PI_2),
            epsilon = eps
        );
        assert_abs_diff_eq!(
            Orientation(5.0 * FRAC_PI_3).angle_to(Orientation(FRAC_PI_3), Direction::Clockwise),
            Angle(4.0 * FRAC_PI_3),
            epsilon = eps
        );
        assert_abs_diff_eq!(
            Orientation(5.0 * FRAC_PI_3)
                .angle_to(Orientation(FRAC_PI_3), Direction::Counterclockwise),
            Angle(2.0 * FRAC_PI_3),
            epsilon = eps
        );
    }

    #[test]
    fn angle_between() {
        let eps = 1e-15;

        assert_abs_diff_eq!(
            Orientation(0.0).angle_between(Orientation(FRAC_PI_2)),
            Angle(FRAC_PI_2),
            epsilon = eps
        );
        assert_abs_diff_eq!(
            Orientation(0.0).angle_between(Orientation(3.0 * FRAC_PI_2)),
            Angle(FRAC_PI_2),
            epsilon = eps
        );
        assert_abs_diff_eq!(
            Orientation(FRAC_PI_4).angle_between(Orientation(-FRAC_PI_4)),
            Angle(FRAC_PI_2),
            epsilon = eps
        );
        assert_abs_diff_eq!(
            Orientation(PI * 2.0).angle_between(Orientation(0.0)),
            Angle(0.0),
            epsilon = eps
        );
    }
}
