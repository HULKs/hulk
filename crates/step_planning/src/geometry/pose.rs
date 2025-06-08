use std::{
    fmt::Debug,
    ops::{Add, AddAssign, Mul},
};

use approx::{AbsDiffEq, RelativeEq};
use nalgebra::{RealField, Scalar};
use num_traits::Euclid;

use coordinate_systems::Ground;
use linear_algebra::{Point2, Rotation2, Vector2};
use types::{step::Step, support_foot::Side};

use crate::geometry::angle::Angle;

#[derive(Clone, Debug)]
pub struct Pose<T: Scalar> {
    pub position: Point2<Ground, T>,
    pub orientation: Angle<T>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PoseGradient<T: Scalar> {
    pub position: Vector2<Ground, T>,
    pub orientation: T,
}

impl<T: RealField + Euclid> PartialEq for Pose<T> {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position && self.orientation == other.orientation
    }
}

impl<T: AbsDiffEq + Euclid + RealField> AbsDiffEq for PoseGradient<T> {
    type Epsilon = T::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        T::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.position.abs_diff_eq(&other.position, epsilon.clone())
            && self.orientation.abs_diff_eq(&other.orientation, epsilon)
    }
}

impl<T: RelativeEq + Euclid + RealField> RelativeEq for PoseGradient<T> {
    fn default_max_relative() -> Self::Epsilon {
        T::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.position
            .relative_eq(&other.position, epsilon.clone(), max_relative.clone())
            && self
                .orientation
                .relative_eq(&other.orientation, epsilon, max_relative)
    }
}

impl<T: Scalar> Pose<T> {
    pub fn with_support_foot(self, support_foot: Side) -> PoseAndSupportFoot<T> {
        PoseAndSupportFoot {
            pose: self,
            support_foot,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PoseAndSupportFoot<T: Scalar> {
    pub pose: Pose<T>,
    pub support_foot: Side,
}

impl<T: RealField> Add<Step<T>> for Pose<T> {
    type Output = Self;

    fn add(self, step: Step<T>) -> Self::Output {
        let Self {
            position,
            orientation,
        } = self;
        let Step {
            forward,
            left,
            turn,
        } = step;

        Self {
            position: position
                + (Rotation2::new(orientation.clone().into_inner())
                    * Vector2::<Ground, T>::wrap(nalgebra::vector![forward, left])),
            orientation: orientation + Angle(turn),
        }
    }
}

impl<T: RealField> AddAssign<Step<T>> for Pose<T> {
    fn add_assign(&mut self, step: Step<T>) {
        *self = self.clone() + step;
    }
}

impl<T: RealField + Copy> Add for PoseGradient<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        let Self {
            position,
            orientation,
        } = self;

        Self {
            position: position + other.position,
            orientation: orientation + other.orientation,
        }
    }
}

impl<T: RealField> Mul<T> for PoseGradient<T> {
    type Output = Self;

    fn mul(self, scale: T) -> Self::Output {
        let Self {
            position,
            orientation,
        } = self;

        Self {
            position: position * scale.clone(),
            orientation: orientation * scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use linear_algebra::{point, Point2};
    use types::step::Step;

    use crate::geometry::{angle::Angle, Pose};

    #[test]
    fn test_pose_step_addition() {
        let pose = Pose {
            position: Point2::origin(),
            orientation: Angle(0.0),
        };
        let step = Step {
            forward: 2.0,
            left: 1.0,
            turn: 3.0,
        };
        let new_pose = pose + step;
        assert_eq!(
            new_pose,
            Pose {
                position: point![2.0, 1.0],
                orientation: Angle(3.0)
            }
        );
    }
}
