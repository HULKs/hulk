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
