use std::{
    fmt::Debug,
    ops::{Add, AddAssign, Mul},
};

use nalgebra::{RealField, Scalar};
use num_traits::Euclid;

use coordinate_systems::Ground;
use linear_algebra::{Point2, Rotation2, Vector2};
use types::{step::Step, support_foot::Side};

#[derive(Clone, Debug)]
pub struct Pose<T: Scalar> {
    pub position: Point2<Ground, T>,
    pub orientation: T,
}

impl<T: RealField + Euclid> PartialEq for Pose<T> {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position && self.orientation == other.orientation
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
                + (Rotation2::new(orientation.clone())
                    * Vector2::<Ground, T>::wrap(nalgebra::vector![forward, left])),
            orientation: orientation + turn,
        }
    }
}

impl<T: RealField> AddAssign<Step<T>> for Pose<T> {
    fn add_assign(&mut self, step: Step<T>) {
        *self = self.clone() + step;
    }
}

impl<T: RealField + Copy> Add for Pose<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        let Self {
            position,
            orientation,
        } = self;

        Self {
            position: position + other.position.coords(),
            orientation: orientation + other.orientation,
        }
    }
}

impl<T: RealField> Mul<T> for Pose<T> {
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
