use std::time::Duration;

use coordinate_systems::Walk;
use linear_algebra::Vector2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{step::Step, support_foot::Side};

use crate::Context;

use super::{anatomic_constraints::AnatomicConstraints, feet::Feet};

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct StepPlan {
    pub step_duration: Duration,
    pub start_feet: Feet,
    pub end_feet: Feet,
    pub support_side: Side,
    pub foot_lift_apex: f32,
    pub midpoint: f32,
}

impl StepPlan {
    pub fn new_from_request(context: &Context, requested_step: Step, support_side: Side) -> Self {
        let parameters = &context.parameters;
        let start_feet = Feet::from_joints(
            context.robot_to_walk,
            &context.last_actuated_joints,
            support_side,
        );

        let step = requested_step.clamp_to_anatomic_constraints(
            support_side,
            parameters.max_base_inside_turn,
            parameters.max_inside_turn_increase,
        );
        let end_feet = Feet::end_from_request(parameters, step, support_side);

        let swing_foot_travel = start_feet.swing_travel_over_ground(&end_feet).abs();
        let turn_travel = end_feet
            .swing_sole
            .orientation()
            .angle_to(start_feet.swing_sole.orientation());

        let foot_lift_apex = parameters.base.foot_lift_apex
            + travel_weighting(
                swing_foot_travel,
                turn_travel,
                parameters
                    .base
                    .foot_lift_apex_increase
                    .div_or_zero(context.max_step_size),
            );

        let step_duration = parameters.base.step_duration
            + Duration::from_secs_f32(travel_weighting(
                swing_foot_travel,
                turn_travel,
                parameters
                    .base
                    .step_duration_increase
                    .div_or_zero(context.max_step_size),
            ));

        let midpoint = interpolate_midpoint(
            swing_foot_travel,
            parameters.step_midpoint,
            parameters.base.step_midpoint,
        );

        StepPlan {
            step_duration,
            start_feet,
            end_feet,
            support_side,
            foot_lift_apex,
            midpoint,
        }
    }
}

fn interpolate_midpoint(
    swing_foot_travel: Vector2<Walk>,
    target_midpoints: Step,
    base_midpoint: f32,
) -> f32 {
    match swing_foot_travel.try_normalize(f32::EPSILON) {
        Some(travel) => travel.x() * target_midpoints.forward + travel.y() * target_midpoints.left,
        None => base_midpoint,
    }
}

fn travel_weighting(translation_travel: Vector2<Walk>, turn_travel: f32, factors: Step) -> f32 {
    let translational = nalgebra::vector![
        factors.forward * translation_travel.x(),
        factors.left * translation_travel.y(),
    ]
    .norm();
    let rotational = factors.turn * turn_travel;
    translational + rotational
}
