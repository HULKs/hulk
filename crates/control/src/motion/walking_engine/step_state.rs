use std::{
    f32::EPSILON,
    time::{Duration, SystemTime},
};

use coordinate_systems::Walk;
use linear_algebra::{point, Point3};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use splines::Interpolate;
use types::{step_plan::Step, support_foot::Side, walking_engine::WalkingEngineParameters};

use super::{
    anatomic_constraints::AnatomicConstraints,
    feet::{parabolic_return, parabolic_step, Feet},
    CycleContext,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct StepState {
    pub started_at: SystemTime,
    pub step_duration: Duration,
    pub start_feet: Feet,
    pub end_feet: Feet,
    pub support_side: Side,
    pub max_swing_foot_lift: f32,
    pub midpoint: f32,
}

impl StepState {
    pub fn new(
        context: &CycleContext,
        requested_step: Step,
        support_side: Side,
        start_feet: Feet,
    ) -> Self {
        let started_at = context.cycle_time.start_time - context.cycle_time.last_cycle_duration;
        let step = requested_step
            .clamp_to_anatomic_constraints(support_side, context.parameters.max_inside_turn);
        let end_feet = Feet::end_from_request(context, step, support_side);

        let swing_foot_travel = start_feet.swing_travel_over_ground(&end_feet).abs();
        let turn_travel = (end_feet.swing_turn - start_feet.swing_turn).abs();

        let base_parameters = &context.parameters.base;
        let max_swing_foot_lift = base_parameters.foot_lift_apex
            + nalgebra::vector![
                base_parameters.foot_lift_apex_increase.forward * swing_foot_travel.x(),
                base_parameters.foot_lift_apex_increase.left * swing_foot_travel.y(),
                base_parameters.foot_lift_apex_increase.turn * turn_travel
            ]
            .norm();

        let step_duration = base_parameters.step_duration
            + Duration::from_secs_f32(
                nalgebra::vector![
                    base_parameters.step_duration_increase.forward * swing_foot_travel.x(),
                    base_parameters.step_duration_increase.left * swing_foot_travel.y(),
                    base_parameters.step_duration_increase.turn * turn_travel
                ]
                .norm(),
            );

        let midpoint = match swing_foot_travel.try_normalize(EPSILON) {
            Some(travel) => {
                travel.x() * context.parameters.step_midpoint.forward
                    + travel.y() * context.parameters.step_midpoint.left
            }
            None => base_parameters.step_midpoint,
        };

        StepState {
            started_at,
            step_duration,
            start_feet,
            end_feet,
            support_side,
            max_swing_foot_lift,
            midpoint,
        }
    }

    pub fn time_since_start(&self, t: SystemTime) -> Duration {
        t.duration_since(self.started_at).unwrap_or_default()
    }

    pub fn normalized_time_since_start(&self, t: SystemTime) -> f32 {
        self.time_since_start(t).as_secs_f32() / self.step_duration.as_secs_f32()
    }

    pub fn is_finished(&self, context: &CycleContext) -> bool {
        let now = context.cycle_time.start_time;
        self.time_since_start(now) > self.step_duration
    }

    pub fn is_support_switched(&self, context: &CycleContext) -> bool {
        let now = context.cycle_time.start_time;
        let pressure_left = context.sensor_data.force_sensitive_resistors.left.sum()
            > context.parameters.sole_pressure_threshold;
        let pressure_right = context.sensor_data.force_sensitive_resistors.right.sum()
            > context.parameters.sole_pressure_threshold;

        let minimal_time = self.time_since_start(now) > context.parameters.min_step_duration;
        let is_support_switched = match self.support_side {
            Side::Left => pressure_right,
            Side::Right => pressure_left,
        };

        minimal_time && is_support_switched
    }

    pub fn is_timeouted(&self, context: &CycleContext) -> bool {
        let now = context.cycle_time.start_time;
        self.time_since_start(now) > context.parameters.max_step_duration
    }

    pub fn feet_at(&self, t: SystemTime, parameters: &WalkingEngineParameters) -> Feet {
        let support_foot = self.support_foot_at(t, parameters);
        let swing_foot = self.swing_foot_at(t);
        let swing_turn = self.swing_turn_at(t);
        Feet {
            support_foot,
            swing_foot,
            swing_turn,
        }
    }

    pub fn support_foot_at(
        &self,
        t: SystemTime,
        parameters: &WalkingEngineParameters,
    ) -> Point3<Walk> {
        let normalized_time = self.normalized_time_since_start(t).clamp(0.0, 1.0);
        let start = self.start_feet.support_foot;
        let end = self.end_feet.support_foot;

        let start_offsets = start.xy();
        let end_offsets = end.xy();
        let offsets = start_offsets.lerp(end_offsets, normalized_time);

        let time_since_start = self.time_since_start(t);
        let max_lift_speed = parameters.max_support_foot_lift_speed;
        let max_lift_delta = time_since_start.as_secs_f32() * max_lift_speed;
        let start_lift = start.z();
        let end_lift = end.z();
        let lift = start_lift + (end_lift - start_lift).clamp(-max_lift_delta, max_lift_delta);

        point![offsets.x(), offsets.y(), lift]
    }

    pub fn swing_foot_at(&self, t: SystemTime) -> Point3<Walk> {
        let normalized_time = self.normalized_time_since_start(t).clamp(0.0, 1.0);
        let parabolic_time = parabolic_step(normalized_time);
        let start = self.start_feet.swing_foot;
        let end = self.end_feet.swing_foot;
        let interpolated = start.lerp(end, parabolic_time);
        let lift = self.swing_foot_lift_at(t);

        point![interpolated.x(), interpolated.y(), lift]
    }

    fn swing_foot_lift_at(&self, t: SystemTime) -> f32 {
        let normalized_time = self.normalized_time_since_start(t).clamp(0.0, 1.0);
        let parabolic_time = parabolic_return(normalized_time, self.midpoint);
        self.max_swing_foot_lift * parabolic_time
    }

    fn swing_turn_at(&self, t: SystemTime) -> f32 {
        let normalized_time = self.normalized_time_since_start(t).clamp(0.0, 1.0);
        let start = self.start_feet.swing_turn;
        let target = self.end_feet.swing_turn;
        f32::lerp(normalized_time, start, target)
    }
}
