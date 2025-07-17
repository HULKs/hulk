use std::ops::Mul;

use nalgebra::RealField;
use num_dual::DualNum;
use types::{step::Step, support_foot::Side, walk_volume_extents::WalkVolumeExtents};

// normalized such that the walk volume is the unit sphere
#[derive(Clone, Debug)]
pub struct NormalizedStep<T> {
    pub forward: T,
    pub left: T,
    pub turn: T,
}

impl<T: RealField + DualNum<f32>> NormalizedStep<T> {
    pub fn unnormalize(
        &self,
        walk_volume_extents: &WalkVolumeExtents,
        support_side: Side,
    ) -> Step<T> {
        let (left_extent, right_extent, clockwise_turn_extent, counterclockwise_turn_extent) =
            match support_side {
                Side::Left => (
                    walk_volume_extents.inward,
                    walk_volume_extents.outward,
                    walk_volume_extents.inward_rotation,
                    walk_volume_extents.outward_rotation,
                ),
                Side::Right => (
                    walk_volume_extents.outward,
                    walk_volume_extents.inward,
                    walk_volume_extents.outward_rotation,
                    walk_volume_extents.inward_rotation,
                ),
            };

        let forward_factor = if self.forward.is_sign_positive() {
            walk_volume_extents.forward
        } else {
            walk_volume_extents.backward
        };

        let left_factor = if self.left.is_sign_positive() {
            left_extent
        } else {
            right_extent
        };

        let turn_factor = if self.turn.is_sign_positive() {
            counterclockwise_turn_extent
        } else {
            clockwise_turn_extent
        };

        Step {
            forward: self.forward.clone() * forward_factor,
            left: self.left.clone() * left_factor,
            turn: self.turn.clone() * turn_factor,
        }
    }

    pub fn from_step(
        step: Step<T>,
        walk_volume_extents: &WalkVolumeExtents,
        support_side: Side,
    ) -> Self {
        let (left_extent, right_extent, clockwise_turn_extent, counterclockwise_turn_extent) =
            match support_side {
                Side::Left => (
                    walk_volume_extents.inward,
                    walk_volume_extents.outward,
                    walk_volume_extents.inward_rotation,
                    walk_volume_extents.outward_rotation,
                ),
                Side::Right => (
                    walk_volume_extents.outward,
                    walk_volume_extents.inward,
                    walk_volume_extents.outward_rotation,
                    walk_volume_extents.inward_rotation,
                ),
            };

        let forward_factor = if step.forward.is_sign_positive() {
            walk_volume_extents.forward
        } else {
            walk_volume_extents.backward
        };

        let left_factor = if step.left.is_sign_positive() {
            left_extent
        } else {
            right_extent
        };

        let turn_factor = if step.turn.is_sign_positive() {
            counterclockwise_turn_extent
        } else {
            clockwise_turn_extent
        };

        Self {
            forward: step.forward / forward_factor,
            left: step.left / left_factor,
            turn: step.turn / turn_factor,
        }
    }

    pub fn is_inside_walk_volume(&self) -> bool {
        let Self {
            forward,
            left,
            turn,
        } = self;

        [forward, left, turn]
            .into_iter()
            .map(|x| x.powi(2))
            .sum::<T>()
            <= T::one()
    }

    pub fn clamp_to_walk_volume(self) -> Self {
        let Self {
            forward,
            left,
            turn,
        } = self;

        let squared_magnitude = [&forward, &left, &turn]
            .iter()
            .map(|x| x.powi(2))
            .sum::<T>();

        if squared_magnitude > T::one() {
            let magnitude = squared_magnitude.sqrt();
            let factor = magnitude.recip();

            Self {
                forward: forward * factor.clone(),
                left: left * factor.clone(),
                turn: turn * factor,
            }
        } else {
            Self {
                forward,
                left,
                turn,
            }
        }
    }
}

impl<T: Clone> NormalizedStep<T> {
    pub fn from_slice(slice: &[T]) -> Self {
        let [forward, left, turn]: &[T; 3] = slice.try_into().unwrap();

        Self {
            forward: forward.clone(),
            left: left.clone(),
            turn: turn.clone(),
        }
    }
}

impl<T: Mul<Output = T> + Clone> Mul<T> for NormalizedStep<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self {
            forward: self.forward * rhs.clone(),
            left: self.left * rhs.clone(),
            turn: self.turn * rhs,
        }
    }
}
