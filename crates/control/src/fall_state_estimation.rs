use std::{ops::Range, time::Duration};

use color_eyre::Result;
use coordinate_systems::Robot;
use linear_algebra::{vector, Isometry3, Vector2, Vector3};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use types::{
    cycle_time::CycleTime,
    fall_state::{Direction, FallState, Side, Variant},
    joints::body::LowerBodyJoints,
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
    upright_range: Parameter<Range<f32>, "fall_state_estimation.upright_range">,
    moving_velocity_threshold: Parameter<f32, "fall_state_estimation.moving_velocity_threshold">,
    falling_timeout: Parameter<Duration, "fall_state_estimation.falling_timeout">,
    fallen_acceleration_threshold:
        Parameter<f32, "fall_state_estimation.fallen_acceleration_threshold">,
    fallen_sitting_rotation: Parameter<f32, "fall_state_estimation.fallen_sitting_rotation">,
    fallen_squatting_joints:
        Parameter<LowerBodyJoints<f32>, "fall_state_estimation.fallen_squatting_joints">,
    joints_difference_threshold:
        Parameter<f32, "fall_state_estimation.joints_difference_threshold">,

    sensor_data: Input<SensorData, "sensor_data">,
    cycle_time: Input<CycleTime, "cycle_time">,
    has_ground_contact: Input<bool, "has_ground_contact">,
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

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let inertial_measurement_unit = context.sensor_data.inertial_measurement_unit;

        self.roll_pitch_filter
            .update(inertial_measurement_unit.roll_pitch);
        self.angular_velocity_filter
            .update(inertial_measurement_unit.angular_velocity);
        self.linear_acceleration_filter
            .update(inertial_measurement_unit.linear_acceleration);

        let estimated_roll = self.roll_pitch_filter.state().x();
        let estimated_pitch = self.roll_pitch_filter.state().y();
        let estimated_angular_velocity = self.angular_velocity_filter.state();
        let estimated_acceleration = self.linear_acceleration_filter.state();
        let is_upright = context.upright_range.contains(&estimated_pitch);
        let is_moving = estimated_angular_velocity.norm() > *context.moving_velocity_threshold;

        let falling_direction = is_falling(is_upright, is_moving, estimated_roll, estimated_pitch);

        let fallen_variant = estimate_fallen_variant(&context, is_moving, estimated_acceleration);

        let fall_state = match (self.last_fall_state, falling_direction, fallen_variant) {
            (FallState::Upright, None, _) => FallState::Upright,
            (FallState::Upright, Some(direction), _) => FallState::Falling {
                direction,
                start_time: context.cycle_time.start_time,
            },
            (current @ FallState::Falling { start_time, .. }, _, Some(facing)) => {
                if context
                    .cycle_time
                    .start_time
                    .duration_since(start_time)
                    .unwrap()
                    > *context.falling_timeout
                {
                    FallState::Fallen { variant: facing }
                } else {
                    current
                }
            }
            (current @ FallState::Falling { start_time, .. }, _, None) => {
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
            (FallState::Fallen { .. }, _, Some(variant)) => FallState::Fallen { variant },
            (FallState::Fallen { variant }, _, None) => FallState::StandingUp { variant },
            (FallState::StandingUp { .. }, _, None) if is_upright => FallState::Upright,
            (current @ FallState::StandingUp { .. }, _, None) => current,
            (FallState::StandingUp { .. }, _, Some(variant)) => FallState::Fallen { variant },
        };

        self.last_fall_state = fall_state;

        Ok(MainOutputs {
            fall_state: fall_state.into(),
        })
    }
}

fn is_falling(
    is_upright: bool,
    is_moving: bool,
    estimated_roll: f32,
    estimated_pitch: f32,
) -> Option<Direction> {
    if is_upright || !is_moving {
        return None;
    }
    let side = {
        if estimated_roll > 0.0 {
            Side::Right
        } else {
            Side::Left
        }
    };
    if estimated_pitch > 0.0 {
        Some(Direction::Forward { side })
    } else {
        Some(Direction::Backward { side })
    }
}

fn estimate_fallen_variant(
    context: &CycleContext,
    is_moving: bool,
    estimated_acceleration: Vector3<Robot>,
) -> Option<Variant> {
    if is_moving {
        return None;
    }

    const GRAVITATIONAL_CONSTANT: f32 = 9.81;
    let acceleration_front = vector![-GRAVITATIONAL_CONSTANT, 0.0, 0.0];
    let acceleration_back = vector![GRAVITATIONAL_CONSTANT, 0.0, 0.0];
    let acceleration_sitting =
        Isometry3::<Robot, _>::rotation(Vector3::y_axis() * *context.fallen_sitting_rotation)
            * vector![0.0, 0.0, GRAVITATIONAL_CONSTANT];

    let fallen_front_difference = (estimated_acceleration - acceleration_front).norm();
    let fallen_back_difference = (estimated_acceleration - acceleration_back).norm();
    let fallen_sitting_difference = (estimated_acceleration - acceleration_sitting).norm();

    let legs_squatting = (*context.fallen_squatting_joints
        - context.sensor_data.positions.body().lower())
    .into_iter()
    .all(|joint| joint.abs() < *context.joints_difference_threshold);

    if *context.has_ground_contact && legs_squatting {
        Some(Variant::Squatting)
    } else if fallen_front_difference < *context.fallen_acceleration_threshold {
        Some(Variant::Front)
    } else if fallen_back_difference < *context.fallen_acceleration_threshold {
        Some(Variant::Back)
    } else if fallen_sitting_difference < *context.fallen_acceleration_threshold {
        Some(Variant::Sitting)
    } else {
        Some(Variant::Unknown)
    }
}
