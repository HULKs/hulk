use std::{
    f32::consts::{PI, TAU},
    ops::{Add, Sub},
};

use geometry::direction::Direction;

use crate::geometry::angle::Angle;

pub fn angle_penalty(current: Angle<f32>, target: Angle<f32>) -> f32 {
    current.angle_between(target).into_inner().powi(2)
}

pub fn angle_penalty_derivative(current: Angle<f32>, target: Angle<f32>) -> f32 {
    let counterclockwise_difference = current
        .angle_to(target, Direction::Counterclockwise)
        .into_inner();

    let minimal_rotation = if counterclockwise_difference > PI {
        counterclockwise_difference - TAU
    } else {
        counterclockwise_difference
    };

    -minimal_rotation * 2.0
}

pub fn angle_penalty_with_tolerance(
    current: Angle<f32>,
    target: Angle<f32>,
    tolerance: f32,
) -> f32 {
    current
        .angle_between(target)
        .into_inner()
        .sub(tolerance)
        .max(0.0)
        .powi(2)
}

pub fn angle_penalty_with_tolerance_derivative(
    current: Angle<f32>,
    target: Angle<f32>,
    tolerance: f32,
) -> f32 {
    let counterclockwise_difference = current
        .angle_to(target, Direction::Counterclockwise)
        .into_inner();

    let minimal_rotation = if counterclockwise_difference > PI {
        (counterclockwise_difference - TAU).add(tolerance).min(0.0)
    } else {
        counterclockwise_difference.sub(tolerance).max(0.0)
    };

    -minimal_rotation * 2.0
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{PI, TAU};

    use proptest::proptest;

    use crate::{
        geometry::angle::Angle,
        utils::angle_penalty::{
            angle_penalty, angle_penalty_derivative, angle_penalty_with_tolerance,
            angle_penalty_with_tolerance_derivative,
        },
    };

    proptest!(
        #[test]
        fn verify_angle_penalty_gradient(orientation in 0.0..TAU, target_orientation in 0.0..TAU) {
            let orientation = Angle(orientation);
            let target_orientation = Angle(target_orientation);

            crate::test_utils::verify_gradient::verify_gradient(
                &|orientation| angle_penalty(orientation, target_orientation),
                &|orientation| angle_penalty_derivative(orientation, target_orientation),
                0.05,
                orientation,
            )
        }
    );

    proptest!(
        #[test]
        fn verify_angle_penalty_with_tolerance_gradient(orientation in 0.0..TAU, target_orientation in 0.0..TAU, tolerance in 0.0..PI) {
            let orientation = Angle(orientation);
            let target_orientation = Angle(target_orientation);

            crate::test_utils::verify_gradient::verify_gradient(
                &|orientation| angle_penalty_with_tolerance(orientation, target_orientation, tolerance),
                &|orientation| angle_penalty_with_tolerance_derivative(orientation, target_orientation, tolerance),
                0.05,
                orientation,
            )
        }
    );
}
