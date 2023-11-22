use std::{f32::consts::FRAC_PI_2, time::Duration};

use color_eyre::Result;
use nalgebra::{vector, Rotation3, Vector2, Vector3};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use filtering::low_pass_filter::LowPassFilter;
use framework::{AdditionalOutput, MainOutput};
use types::{
    cycle_time::CycleTime,
    fall_state::{Facing, FallDirection, FallState, Side},
    sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct FallStateEstimation {
    roll_pitch_filter: LowPassFilter<Vector2<f32>>,
    angular_velocity_filter: LowPassFilter<Vector3<f32>>,
    linear_acceleration_filter: LowPassFilter<Vector3<f32>>,
    last_fall_state: FallState,
}

#[context]
pub struct CreationContext {
    linear_acceleration_low_pass_factor:
        Parameter<f32, "fall_state_estimation.linear_acceleration_low_pass_factor">,
    angular_velocity_low_pass_factor:
        Parameter<f32, "fall_state_estimation.angular_velocity_low_pass_factor">,
    roll_pitch_low_pass_factor: Parameter<f32, "fall_state_estimation.roll_pitch_low_pass_factor">,
}

#[context]
pub struct CycleContext {
    fallen_up_gravitational_difference: AdditionalOutput<f32, "fallen_up_gravitational_difference">,
    filtered_angular_velocity: AdditionalOutput<Vector3<f32>, "filtered_angular_velocity">,
    filtered_linear_acceleration: AdditionalOutput<Vector3<f32>, "filtered_linear_acceleration">,
    filtered_roll_pitch: AdditionalOutput<Vector2<f32>, "filtered_roll_pitch">,
    fallen_down_gravitational_difference:
        AdditionalOutput<f32, "fallen_down_gravitational_difference">,

    gravitational_acceleration_threshold:
        Parameter<f32, "fall_state_estimation.gravitational_acceleration_threshold">,
    falling_angle_threshold_forward:
        Parameter<Vector2<f32>, "fall_state_estimation.falling_angle_threshold_forward">,
    falling_timeout: Parameter<Duration, "fall_state_estimation.falling_timeout">,

    sensor_data: Input<SensorData, "sensor_data">,
    cycle_time: Input<CycleTime, "cycle_time">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub fall_state: MainOutput<FallState>,
}

impl FallStateEstimation {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            roll_pitch_filter: LowPassFilter::with_smoothing_factor(
                Vector2::zeros(),
                *context.roll_pitch_low_pass_factor,
            ),
            angular_velocity_filter: LowPassFilter::with_smoothing_factor(
                Vector3::zeros(),
                *context.angular_velocity_low_pass_factor,
            ),
            linear_acceleration_filter: LowPassFilter::with_smoothing_factor(
                Vector3::zeros(),
                *context.linear_acceleration_low_pass_factor,
            ),
            last_fall_state: Default::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let inertial_measurement_unit = context.sensor_data.inertial_measurement_unit;

        self.roll_pitch_filter
            .update(inertial_measurement_unit.roll_pitch);
        self.angular_velocity_filter
            .update(inertial_measurement_unit.angular_velocity);
        self.linear_acceleration_filter
            .update(inertial_measurement_unit.linear_acceleration);

        context
            .filtered_roll_pitch
            .fill_if_subscribed(|| self.roll_pitch_filter.state());
        context
            .filtered_linear_acceleration
            .fill_if_subscribed(|| self.linear_acceleration_filter.state());
        context
            .filtered_angular_velocity
            .fill_if_subscribed(|| self.angular_velocity_filter.state());

        const GRAVITATIONAL_CONSTANT: f32 = 9.81;

        let gravitational_force = vector![0.0, 0.0, GRAVITATIONAL_CONSTANT];
        let robot_to_fallen_down = Rotation3::from_axis_angle(&Vector3::y_axis(), -FRAC_PI_2);
        let robot_to_fallen_up = Rotation3::from_axis_angle(&Vector3::y_axis(), FRAC_PI_2);

        let fallen_down_gravitational_difference = (self.linear_acceleration_filter.state()
            - robot_to_fallen_down * gravitational_force)
            .norm();
        let fallen_up_gravitational_difference = (self.linear_acceleration_filter.state()
            - robot_to_fallen_up * gravitational_force)
            .norm();
        let fallen_direction = if fallen_down_gravitational_difference
            < *context.gravitational_acceleration_threshold
        {
            Some(Facing::Down)
        } else if fallen_up_gravitational_difference < *context.gravitational_acceleration_threshold
        {
            Some(Facing::Up)
        } else {
            None
        };
        context
            .fallen_down_gravitational_difference
            .fill_if_subscribed(|| fallen_down_gravitational_difference);
        context
            .fallen_up_gravitational_difference
            .fill_if_subscribed(|| fallen_up_gravitational_difference);

        let estimated_roll = self.roll_pitch_filter.state().x;
        let estimated_pitch = self.roll_pitch_filter.state().y;

        let falling_direction = {
            if !(context.falling_angle_threshold_forward[0]
                ..context.falling_angle_threshold_forward[1])
                .contains(&estimated_pitch)
            {
                let side = {
                    if estimated_roll > 0.0 {
                        Side::Right
                    } else {
                        Side::Left
                    }
                };
                if estimated_pitch > 0.0 {
                    Some(FallDirection::Forward { side })
                } else {
                    Some(FallDirection::Backward { side })
                }
            } else {
                None
            }
        };

        let fall_state = match (self.last_fall_state, falling_direction, fallen_direction) {
            (FallState::Upright, None, None) => FallState::Upright,
            (FallState::Upright, None, Some(facing)) => FallState::Fallen { facing },
            (FallState::Upright, Some(direction), None) => FallState::Falling {
                direction,
                start_time: context.cycle_time.start_time,
            },
            (FallState::Upright, Some(_), Some(facing)) => FallState::Fallen { facing },
            (current @ FallState::Falling { start_time, .. }, None, None) => {
                if context
                    .cycle_time
                    .start_time
                    .duration_since(start_time)
                    .unwrap()
                    > *context.falling_timeout
                {
                    FallState::Upright
                } else {
                    current
                }
            }
            (current @ FallState::Falling { start_time, .. }, None | Some(_), Some(facing)) => {
                if context
                    .cycle_time
                    .start_time
                    .duration_since(start_time)
                    .unwrap()
                    > *context.falling_timeout
                {
                    FallState::Fallen { facing }
                } else {
                    current
                }
            }
            (current @ FallState::Falling { .. }, Some(_), None) => current,
            (FallState::Fallen { .. }, None, None) => FallState::Upright,
            (FallState::Fallen { .. }, None | Some(_), Some(facing)) => {
                FallState::Fallen { facing }
            }
            (FallState::Fallen { facing }, Some(_), None) => FallState::Fallen { facing },
        };

        self.last_fall_state = fall_state;

        Ok(MainOutputs {
            fall_state: fall_state.into(),
        })
    }
}
