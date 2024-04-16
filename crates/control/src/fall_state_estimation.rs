use std::time::Duration;

use color_eyre::Result;
use coordinate_systems::Robot;
use linear_algebra::{vector, Vector2, Vector3};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use filtering::low_pass_filter::LowPassFilter;
use framework::{AdditionalOutput, MainOutput};
use types::{
    cycle_time::CycleTime,
    fall_state::{FallDirection, FallState, Orientation, Side},
    joints::Joints,
    sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct FallStateEstimation {
    roll_pitch_filter: LowPassFilter<Vector2<Robot>>,
    angular_velocity_filter: LowPassFilter<Vector3<Robot>>,
    linear_acceleration_filter: LowPassFilter<Vector3<Robot>>,
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
    filtered_angular_velocity: AdditionalOutput<Vector3<Robot>, "filtered_angular_velocity">,
    filtered_linear_acceleration: AdditionalOutput<Vector3<Robot>, "filtered_linear_acceleration">,
    filtered_roll_pitch: AdditionalOutput<Vector2<Robot>, "filtered_roll_pitch">,
    fallen_down_gravitational_difference:
        AdditionalOutput<f32, "fallen_down_gravitational_difference">,
    fallen_standing_gravitational_difference:
        AdditionalOutput<f32, "fallen_standing_gravitational_difference">,
    fallen_up_gravitational_difference: AdditionalOutput<f32, "fallen_up_gravitational_difference">,
    difference_to_sitting: AdditionalOutput<f32, "difference_to_sitting">,

    gravitational_acceleration_threshold:
        Parameter<f32, "fall_state_estimation.gravitational_acceleration_threshold">,
    falling_angle_threshold_forward:
        Parameter<Vector2<Robot>, "fall_state_estimation.falling_angle_threshold_forward">,
    difference_to_sitting_threshold:
        Parameter<f32, "fall_state_estimation.difference_to_sitting_threshold">,
    falling_timeout: Parameter<Duration, "fall_state_estimation.falling_timeout">,
    sitting_pose: Parameter<Joints<f32>, "fall_state_estimation.sitting_pose">,

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

        let gravitational_force_down = vector![-GRAVITATIONAL_CONSTANT, 0.0, 0.0];
        let gravitational_force_up = vector![GRAVITATIONAL_CONSTANT, 0.0, 0.0];
        let graviational_force_upright = vector![3.6, 0.0, 8.8];

        let fallen_down_gravitational_difference =
            (self.linear_acceleration_filter.state() - gravitational_force_down).norm();
        let fallen_up_gravitational_difference =
            (self.linear_acceleration_filter.state() - gravitational_force_up).norm();
        let fallen_standing_gravitational_difference =
            (self.linear_acceleration_filter.state() - graviational_force_upright).norm();

        let positions = context.sensor_data.positions;
        let difference_to_sitting: f32 = (*context.sitting_pose - positions)
            .into_iter()
            .map(|position| position.powf(2.0))
            .sum();
        let fallen_direction = if fallen_down_gravitational_difference
            < *context.gravitational_acceleration_threshold
        {
            Some(Orientation::FacingDown)
        } else if fallen_up_gravitational_difference < *context.gravitational_acceleration_threshold
        {
            Some(Orientation::FacingUp)
        } else if fallen_standing_gravitational_difference
            < *context.gravitational_acceleration_threshold
            && difference_to_sitting < *context.difference_to_sitting_threshold
        {
            Some(Orientation::Sitting)
        } else {
            None
        };
        context
            .fallen_down_gravitational_difference
            .fill_if_subscribed(|| fallen_down_gravitational_difference);
        context
            .fallen_up_gravitational_difference
            .fill_if_subscribed(|| fallen_up_gravitational_difference);
        context
            .fallen_standing_gravitational_difference
            .fill_if_subscribed(|| fallen_standing_gravitational_difference);
        context
            .difference_to_sitting
            .fill_if_subscribed(|| difference_to_sitting);

        let estimated_roll = self.roll_pitch_filter.state().x();
        let estimated_pitch = self.roll_pitch_filter.state().y();

        let falling_direction = {
            if !(context.falling_angle_threshold_forward.x()
                ..context.falling_angle_threshold_forward.y())
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
            (FallState::Upright, None, Some(facing)) => FallState::Fallen {
                orientation: facing,
            },
            (FallState::Upright, Some(direction), None) => FallState::Falling {
                direction,
                start_time: context.cycle_time.start_time,
            },
            //(FallState::Upright, Some(_), None) => FallState::Upright,
            (FallState::Upright, Some(_), Some(facing)) => FallState::Fallen {
                orientation: facing,
            },
            (current @ FallState::Falling { start_time, .. }, None, None) => {
                if context
                    .cycle_time
                    .start_time
                    .duration_since(start_time)
                    .unwrap()
                    > *context.falling_timeout
                    && fallen_standing_gravitational_difference
                        < *context.gravitational_acceleration_threshold
                {
                    FallState::Upright
                } else {
                    current
                }
            }
            (current @ FallState::Falling { start_time, .. }, _, Some(facing)) => {
                if context
                    .cycle_time
                    .start_time
                    .duration_since(start_time)
                    .unwrap()
                    > *context.falling_timeout
                {
                    FallState::Fallen {
                        orientation: facing,
                    }
                } else {
                    current
                }
            }
            (current @ FallState::Falling { start_time, .. }, Some(_), None) => {
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
            (FallState::Fallen { .. }, None, None) => FallState::Upright,
            (FallState::Fallen { .. }, None, Some(_)) => FallState::StandingUp,
            (FallState::StandingUp { .. }, None, None) => FallState::Upright,
            (current @ FallState::StandingUp { .. }, Some(_), None) => current,
            (FallState::StandingUp { .. }, None, Some(facing)) => FallState::Fallen {
                orientation: (facing),
            },
            (FallState::Fallen { .. }, Some(_), None) => FallState::Upright,
            (current @ FallState::Fallen { .. }, Some(_), Some(_)) => current,
            (FallState::StandingUp, Some(_), Some(facing)) => FallState::Fallen {
                orientation: (facing),
            },
        };

        self.last_fall_state = fall_state;

        Ok(MainOutputs {
            fall_state: fall_state.into(),
        })
    }
}
