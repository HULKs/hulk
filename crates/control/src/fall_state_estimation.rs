use std::time::Duration;

use color_eyre::Result;
use coordinate_systems::{Field, Robot};
use linear_algebra::{vector, Orientation3, Vector2, Vector3};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use filtering::low_pass_filter::LowPassFilter;
use framework::{AdditionalOutput, MainOutput};
use types::{
    cycle_time::CycleTime,
    fall_state::{Direction, FallState, Kind, Side},
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
    fallen_sitting_gravitational_difference:
        AdditionalOutput<f32, "fallen_standing_gravitational_difference">,
    fallen_up_gravitational_difference: AdditionalOutput<f32, "fallen_up_gravitational_difference">,
    upright_gravitational_difference: AdditionalOutput<f32, "upright_gravitational_difference">,
    difference_to_sitting: AdditionalOutput<f32, "difference_to_sitting">,
    falling_direction: AdditionalOutput<Option<Direction>, "falling_direction">,
    fallen_direction: AdditionalOutput<Option<Kind>, "fallen_direction">,

    difference_to_sitting_threshold:
        Parameter<f32, "fall_state_estimation.difference_to_sitting_threshold">,
    falling_angle_threshold_forward:
        Parameter<Vector2<Robot>, "fall_state_estimation.falling_angle_threshold_forward">,
    falling_angle_threshold_forward_with_catching_steps: Parameter<
        Vector2<Robot>,
        "fall_state_estimation.falling_angle_threshold_forward_with_catching_steps",
    >,
    falling_timeout: Parameter<Duration, "fall_state_estimation.falling_timeout">,
    gravitational_acceleration_threshold:
        Parameter<f32, "fall_state_estimation.gravitational_acceleration_threshold">,
    gravitational_force_sitting:
        Parameter<Vector3<Robot>, "fall_state_estimation.gravitational_force_sitting">,
    gravity_acceleration: Parameter<f32, "physical_constants.gravity_acceleration">,
    sitting_pose: Parameter<Joints<f32>, "fall_state_estimation.sitting_pose">,
    use_catching_steps: Parameter<bool, "walking_engine.catching_steps.use_catching_steps">,

    robot_orientation: RequiredInput<Option<Orientation3<Field>>, "robot_orientation?">,
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

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let cycle_start = context.cycle_time.start_time;
        let inertial_measurement_unit = context.sensor_data.inertial_measurement_unit;
        let (roll, pitch, _) = context.robot_orientation.inner.euler_angles();
        self.roll_pitch_filter.update(vector![roll, pitch]);
        self.angular_velocity_filter
            .update(inertial_measurement_unit.angular_velocity);
        self.linear_acceleration_filter
            .update(inertial_measurement_unit.linear_acceleration);
        let estimated_roll = self.roll_pitch_filter.state().x();
        let estimated_pitch = self.roll_pitch_filter.state().y();

        let gravitational_force_down = vector![-context.gravity_acceleration, 0.0, 0.0];
        let gravitational_force_up = vector![*context.gravity_acceleration, 0.0, 0.0];
        let gravitational_force_upright = vector![0.0, 0.0, *context.gravity_acceleration];

        let fallen_down_gravitational_difference =
            self.compute_gravitational_difference(gravitational_force_down);
        let fallen_up_gravitational_difference =
            self.compute_gravitational_difference(gravitational_force_up);
        let fallen_sitting_gravitational_difference =
            self.compute_gravitational_difference(*context.gravitational_force_sitting);
        let upright_gravitational_difference =
            self.compute_gravitational_difference(gravitational_force_upright);

        let positions = context.sensor_data.positions;

        let difference_to_sitting = (context.sitting_pose.left_leg.hip_pitch
            - positions.left_leg.hip_pitch)
            .powf(2.0)
            + (context.sitting_pose.left_leg.knee_pitch - positions.left_leg.knee_pitch).powf(2.0)
            + (context.sitting_pose.right_leg.hip_pitch - positions.right_leg.hip_pitch).powf(2.0)
            + (context.sitting_pose.right_leg.knee_pitch - positions.right_leg.knee_pitch)
                .powf(2.0);

        let fallen_direction = if fallen_down_gravitational_difference
            < *context.gravitational_acceleration_threshold
        {
            Some(Kind::FacingDown)
        } else if fallen_sitting_gravitational_difference
            < *context.gravitational_acceleration_threshold
            && difference_to_sitting < *context.difference_to_sitting_threshold
            && !context.has_ground_contact
        {
            Some(Kind::Sitting)
        } else if fallen_up_gravitational_difference < *context.gravitational_acceleration_threshold
        {
            Some(Kind::FacingUp)
        } else {
            None
        };

        let falling_angle_threshold = if *context.use_catching_steps {
            context.falling_angle_threshold_forward_with_catching_steps
        } else {
            context.falling_angle_threshold_forward
        };

        let falling_direction = {
            if !(falling_angle_threshold.x()..falling_angle_threshold.y())
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
                    Some(Direction::Forward { side })
                } else {
                    Some(Direction::Backward { side })
                }
            } else {
                None
            }
        };

        let fall_state = match (self.last_fall_state, falling_direction, fallen_direction) {
            (FallState::Upright, None, None) => FallState::Upright,
            (FallState::Upright, None, Some(facing)) => FallState::Fallen { kind: facing },
            (FallState::Upright, Some(direction), None) => FallState::Falling {
                direction,
                start_time: context.cycle_time.start_time,
            },
            (FallState::Upright, Some(_), Some(facing)) => FallState::Fallen { kind: facing },
            (current @ FallState::Falling { start_time, .. }, None, None) => {
                if cycle_start.duration_since(start_time).unwrap() > *context.falling_timeout {
                    if upright_gravitational_difference
                        < *context.gravitational_acceleration_threshold
                    {
                        FallState::Upright
                    } else {
                        decide_standing_up_direction(
                            &context,
                            fallen_up_gravitational_difference,
                            fallen_down_gravitational_difference,
                        )
                    }
                } else {
                    current
                }
            }
            (current @ FallState::Falling { start_time, .. }, _, Some(facing)) => {
                if cycle_start.duration_since(start_time).unwrap() > *context.falling_timeout {
                    FallState::Fallen { kind: facing }
                } else {
                    current
                }
            }
            (current @ FallState::Falling { start_time, .. }, Some(_), None) => {
                if cycle_start.duration_since(start_time).unwrap() > *context.falling_timeout {
                    decide_standing_up_direction(
                        &context,
                        fallen_up_gravitational_difference,
                        fallen_down_gravitational_difference,
                    )
                } else {
                    current
                }
            }
            (FallState::Fallen { .. }, _, None) => FallState::Upright,
            (FallState::Fallen { kind }, _, Some(_)) => FallState::StandingUp {
                start_time: context.cycle_time.start_time,
                kind,
            },
            (FallState::StandingUp { .. }, None, None) => FallState::Upright,
            (current @ FallState::StandingUp { .. }, Some(..), None) => current,
            (current @ FallState::StandingUp { start_time, .. }, _, Some(facing)) => {
                if cycle_start.duration_since(start_time).unwrap() > *context.falling_timeout {
                    FallState::Fallen { kind: (facing) }
                } else {
                    current
                }
            }
        };

        self.last_fall_state = fall_state;

        context
            .fallen_down_gravitational_difference
            .fill_if_subscribed(|| fallen_down_gravitational_difference);
        context
            .fallen_up_gravitational_difference
            .fill_if_subscribed(|| fallen_up_gravitational_difference);
        context
            .fallen_sitting_gravitational_difference
            .fill_if_subscribed(|| fallen_sitting_gravitational_difference);
        context
            .upright_gravitational_difference
            .fill_if_subscribed(|| upright_gravitational_difference);
        context
            .difference_to_sitting
            .fill_if_subscribed(|| difference_to_sitting);
        context
            .filtered_roll_pitch
            .fill_if_subscribed(|| self.roll_pitch_filter.state());
        context
            .filtered_linear_acceleration
            .fill_if_subscribed(|| self.linear_acceleration_filter.state());
        context
            .filtered_angular_velocity
            .fill_if_subscribed(|| self.angular_velocity_filter.state());
        context
            .falling_direction
            .fill_if_subscribed(|| falling_direction);
        context
            .fallen_direction
            .fill_if_subscribed(|| fallen_direction);

        Ok(MainOutputs {
            fall_state: fall_state.into(),
        })
    }

    fn compute_gravitational_difference(&self, gravitational_force: Vector3<Robot>) -> f32 {
        (self.linear_acceleration_filter.state() - gravitational_force).norm()
    }
}

fn decide_standing_up_direction(
    context: &CycleContext,
    fallen_up_gravitational_difference: f32,
    fallen_down_gravitational_difference: f32,
) -> FallState {
    if fallen_up_gravitational_difference < fallen_down_gravitational_difference {
        FallState::StandingUp {
            start_time: context.cycle_time.start_time,
            kind: Kind::FacingUp,
        }
    } else {
        FallState::StandingUp {
            start_time: context.cycle_time.start_time,
            kind: Kind::FacingDown,
        }
    }
}
