use std::f32::consts::{PI, TAU};

use crate::geometry::angle::Angle;

pub fn angle_penalty(current: Angle<f32>, target: Angle<f32>) -> f32 {
    current.angle_between(target).into_inner().powi(2)
}

pub fn angle_penalty_derivative(current: Angle<f32>, target: Angle<f32>) -> Angle<f32> {
    let counterclockwise_difference = (current - target).normalized();

    if counterclockwise_difference.0 > PI {
        -(Angle(TAU) - counterclockwise_difference) * 2.0
    } else {
        counterclockwise_difference * 2.0
    }
}
