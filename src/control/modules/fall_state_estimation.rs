use std::time::{Duration, SystemTime};

use macros::{module, require_some};
use nalgebra::{Vector2, Vector3};

use crate::types::{Facing, FallDirection, FallState, SensorData};

pub struct FallStateEstimation {
    fallen_time: SystemTime,
    filtered_angle: Vector2<f32>,
    filtered_angular_velocity: Vector3<f32>,
    previous_fall_state: FallState,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[parameter(path = control.fall_state_estimation.low_pass_filter_coefficient, data_type = f32)]
#[parameter(path = control.fall_state_estimation.minimum_angle, data_type = Vector2<f32>)]
#[parameter(path = control.fall_state_estimation.maximum_angle, data_type = Vector2<f32>)]
#[parameter(path = control.fall_state_estimation.minimum_angular_velocity, data_type = Vector2<f32>)]
#[parameter(path = control.fall_state_estimation.maximum_angular_velocity, data_type = Vector2<f32>)]
#[parameter(path = control.fall_state_estimation.linear_acceleration_upright_threshold, data_type = Vector3<f32>)]
#[parameter(path = control.fall_state_estimation.fallen_timeout, data_type = Duration)]
#[main_output(data_type = FallState)]
impl FallStateEstimation {}

impl FallStateEstimation {
    pub fn new() -> Self {
        Self {
            fallen_time: SystemTime::UNIX_EPOCH,
            filtered_angle: Vector2::zeros(),
            filtered_angular_velocity: Vector3::zeros(),
            previous_fall_state: FallState::Upright,
        }
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let inertial_measurement_unit =
            &require_some!(context.sensor_data).inertial_measurement_unit;
        let cycle_start_time = require_some!(context.sensor_data).cycle_info.start_time;

        let low_pass_filter_coefficient = *context.low_pass_filter_coefficient;

        self.filtered_angle = inertial_measurement_unit.roll_pitch * low_pass_filter_coefficient
            + self.filtered_angle * (1.0 - low_pass_filter_coefficient);

        self.filtered_angular_velocity = inertial_measurement_unit.angular_velocity
            * low_pass_filter_coefficient
            + self.filtered_angular_velocity * (1.0 - low_pass_filter_coefficient);

        if inertial_measurement_unit.linear_acceleration.x.abs()
            <= context.linear_acceleration_upright_threshold.x
            && inertial_measurement_unit.linear_acceleration.y.abs()
                <= context.linear_acceleration_upright_threshold.y
            && inertial_measurement_unit.linear_acceleration.z.abs()
                >= context.linear_acceleration_upright_threshold.z
        {
            self.previous_fall_state = FallState::Upright;
        }
        let fall_direction = {
            if self.filtered_angle.x < context.minimum_angle.x
                && self.filtered_angular_velocity.x < context.minimum_angular_velocity.x
            {
                self.fallen_time = cycle_start_time;
                Some(FallDirection::Left)
            } else if self.filtered_angle.x > context.maximum_angle.x
                && self.filtered_angular_velocity.x > context.maximum_angular_velocity.x
            {
                self.fallen_time = cycle_start_time;
                Some(FallDirection::Right)
            } else if self.filtered_angle.y < context.minimum_angle.y
                && self.filtered_angular_velocity.y < context.minimum_angular_velocity.y
            {
                self.fallen_time = cycle_start_time;
                Some(FallDirection::Backward)
            } else if self.filtered_angle.y > context.maximum_angle.y
                && self.filtered_angular_velocity.y > context.maximum_angular_velocity.y
            {
                self.fallen_time = cycle_start_time;
                Some(FallDirection::Forward)
            } else {
                None
            }
        };
        let falling_timed_out = cycle_start_time
            .duration_since(self.fallen_time)
            .expect("Cycle start time before fallen time")
            > *context.fallen_timeout;
        let fall_state = match (self.previous_fall_state, fall_direction, falling_timed_out) {
            (FallState::Upright, None, _) => FallState::Upright,
            (FallState::Upright, Some(direction), _) => FallState::Falling { direction },
            (FallState::Falling { .. }, Some(direction), _)
            | (FallState::Falling { direction }, None, false) => FallState::Falling { direction },
            (FallState::Falling { .. }, None, true) | (FallState::Fallen { .. }, _, _) => {
                if self.filtered_angle.y > 0.0 {
                    FallState::Fallen {
                        facing: Facing::Down,
                    }
                } else {
                    FallState::Fallen { facing: Facing::Up }
                }
            }
        };

        self.previous_fall_state = fall_state;

        Ok(MainOutputs {
            fall_state: Some(fall_state),
        })
    }
}
