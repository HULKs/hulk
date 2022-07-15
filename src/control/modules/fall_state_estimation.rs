use std::f32::consts::{FRAC_PI_2, PI};

use module_derive::module;
use nalgebra::{vector, Isometry3, Translation3, UnitQuaternion, Vector2, Vector3};
use types::{Facing, FallDirection, FallState, InertialMeasurementUnitData, SensorData};

use crate::control::filtering::LowPassFilter;

pub struct FallStateEstimation {
    roll_pitch_filter: LowPassFilter<Vector2<f32>>,
    angular_velocity_filter: LowPassFilter<Vector3<f32>>,
    linear_acceleration_filter: LowPassFilter<Vector3<f32>>,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData, required)]
#[input(path = has_ground_contact, data_type = bool, required)]
#[parameter(path = control.fall_state_estimation, data_type = crate::framework::configuration::FallStateEstimation)]
#[additional_output(path = filtered_linear_acceleration, data_type = Vector3<f32>)]
#[additional_output(path = filtered_angular_velocity, data_type = Vector3<f32>)]
#[additional_output(path = filtered_roll_pitch, data_type = Vector2<f32>)]
#[additional_output(path = forward_gravitational_difference, data_type = f32)]
#[additional_output(path = backward_gravitational_difference, data_type = f32)]
#[main_output(data_type = FallState)]
impl FallStateEstimation {}

impl FallStateEstimation {
    fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            roll_pitch_filter: LowPassFilter::with_alpha(
                Vector2::zeros(),
                context.fall_state_estimation.roll_pitch_low_pass_factor,
            ),
            angular_velocity_filter: LowPassFilter::with_alpha(
                Vector3::zeros(),
                context
                    .fall_state_estimation
                    .angular_velocity_low_pass_factor,
            ),
            linear_acceleration_filter: LowPassFilter::with_alpha(
                Vector3::zeros(),
                context
                    .fall_state_estimation
                    .linear_acceleration_low_pass_factor,
            ),
        })
    }

    fn cycle(&mut self, mut context: CycleContext) -> anyhow::Result<MainOutputs> {
        let inertial_measurement_unit = convert_to_right_handed_coordinate_system(
            context.sensor_data.inertial_measurement_unit,
        );

        let robot_to_inertial_measurement_unit = Isometry3::from_parts(
            Translation3::identity(),
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), PI),
        );
        let inertial_measurement_unit_to_robot = robot_to_inertial_measurement_unit.inverse();

        self.roll_pitch_filter
            .update(inertial_measurement_unit.roll_pitch);

        self.angular_velocity_filter.update(
            inertial_measurement_unit_to_robot * inertial_measurement_unit.angular_velocity,
        );

        self.linear_acceleration_filter.update(
            inertial_measurement_unit_to_robot * inertial_measurement_unit.linear_acceleration,
        );

        context
            .filtered_roll_pitch
            .fill_on_subscription(|| self.roll_pitch_filter.state());
        context
            .filtered_linear_acceleration
            .fill_on_subscription(|| self.linear_acceleration_filter.state());
        context
            .filtered_angular_velocity
            .fill_on_subscription(|| self.angular_velocity_filter.state());

        const GRAVITATIONAL_CONSTANT: f32 = -9.81;
        let gravitational_force = vector![0.0, 0.0, GRAVITATIONAL_CONSTANT];
        let robot_to_fallen_down = Isometry3::from_parts(
            Translation3::identity(),
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -FRAC_PI_2),
        );
        let robot_to_fallen_up = Isometry3::from_parts(
            Translation3::identity(),
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), FRAC_PI_2),
        );

        let fallen_direction = if (self.linear_acceleration_filter.state()
            - robot_to_fallen_down * gravitational_force)
            .norm()
            < context
                .fall_state_estimation
                .gravitational_acceleration_threshold
        {
            Some(Facing::Down)
        } else if (self.linear_acceleration_filter.state()
            - robot_to_fallen_up * gravitational_force)
            .norm()
            < context
                .fall_state_estimation
                .gravitational_acceleration_threshold
        {
            Some(Facing::Up)
        } else {
            None
        };
        context
            .forward_gravitational_difference
            .fill_on_subscription(|| {
                (self.linear_acceleration_filter.state()
                    - robot_to_fallen_down * gravitational_force)
                    .norm()
            });
        context
            .backward_gravitational_difference
            .fill_on_subscription(|| {
                (self.linear_acceleration_filter.state() - robot_to_fallen_up * gravitational_force)
                    .norm()
            });

        let falling_direction = {
            if self.roll_pitch_filter.state().x.abs()
                > context.fall_state_estimation.falling_angle_threshold.x
            {
                if self.roll_pitch_filter.state().x > 0.0 {
                    Some(FallDirection::Right)
                } else {
                    Some(FallDirection::Left)
                }
            } else if self.roll_pitch_filter.state().y.abs()
                > context.fall_state_estimation.falling_angle_threshold.y
            {
                if self.roll_pitch_filter.state().y > 0.0 {
                    Some(FallDirection::Forward)
                } else {
                    Some(FallDirection::Backward)
                }
            } else {
                None
            }
        };
        let fall_state = match (fallen_direction, falling_direction) {
            (Some(facing), _) => FallState::Fallen { facing },
            (None, Some(direction)) => FallState::Falling { direction },
            (None, None) => FallState::Upright,
        };

        Ok(MainOutputs {
            fall_state: Some(fall_state),
        })
    }
}

fn convert_to_right_handed_coordinate_system(
    inertial_measurement_unit: InertialMeasurementUnitData,
) -> InertialMeasurementUnitData {
    InertialMeasurementUnitData {
        linear_acceleration: vector![
            inertial_measurement_unit.linear_acceleration.x,
            -inertial_measurement_unit.linear_acceleration.y,
            inertial_measurement_unit.linear_acceleration.z
        ],
        angular_velocity: vector![
            -inertial_measurement_unit.angular_velocity.x,
            inertial_measurement_unit.angular_velocity.y,
            -inertial_measurement_unit.angular_velocity.z
        ],
        roll_pitch: inertial_measurement_unit.roll_pitch,
    }
}
