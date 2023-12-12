use nalgebra::{Quaternion, UnitQuaternion, Vector3};
use types::orientation_filter::{Parameters, State};

const GRAVITATIONAL_CONSTANT: f32 = 9.81;

/**
Paper <https://www.mdpi.com/1424-8220/15/8/19302/pdf>
Name: Keeping a Good Attitude: A Quaternion-Based Orientation Filter for IMUs and MARGs
This implementation of the orientation filter is based on the paper above.
*/
pub trait OrientationFiltering {
    fn update(
        &mut self,
        measured_acceleration: Vector3<f32>,
        measured_angular_velocity: Vector3<f32>,
        left_force_sensitive_resistor: f32,
        right_force_sensitive_resistor: f32,
        cycle_time: f32,
        parameters: &Parameters,
    );
}

impl OrientationFiltering for State {
    fn update(
        &mut self,
        measured_acceleration: Vector3<f32>,
        measured_angular_velocity: Vector3<f32>,
        left_force_sensitive_resistor: f32,
        right_force_sensitive_resistor: f32,
        cycle_time: f32,
        parameters: &Parameters,
    ) {
        if !self.is_initialized {
            if measured_acceleration.norm() < parameters.falling_threshold {
                return;
            }
            self.orientation = calculate_initial_orientation(measured_acceleration);
            self.is_initialized = true;
            return;
        }

        if is_in_steady_state(
            self.previous_angular_velocity,
            measured_acceleration,
            measured_angular_velocity,
            left_force_sensitive_resistor,
            right_force_sensitive_resistor,
            parameters,
        ) {
            // Section 5.4
            self.angular_velocity_bias += parameters.angular_velocity_bias_weight
                * (measured_angular_velocity - self.angular_velocity_bias);
        } else {
            self.orientation = predict(
                self.orientation,
                measured_angular_velocity,
                self.angular_velocity_bias,
                cycle_time,
            );
            if let Some(correction) = correct(
                self.orientation,
                measured_acceleration,
                parameters.acceleration_weight,
            ) {
                self.orientation *= correction
            }
        }
        self.previous_angular_velocity = measured_angular_velocity;
    }
}

fn predict(
    orientation: UnitQuaternion<f32>,
    measured_angular_velocity: Vector3<f32>,
    angular_velocity_bias: Vector3<f32>,
    cycle_time: f32,
) -> UnitQuaternion<f32> {
    let orientation = orientation.quaternion();
    let angular_velocity = measured_angular_velocity - angular_velocity_bias;
    let angular_rate = Quaternion::<f32>::new(
        0.0,
        angular_velocity.x,
        angular_velocity.y,
        angular_velocity.z,
    );
    // Equation 38
    let angular_rate_derivative = -0.5 * (angular_rate * orientation);
    // Equation 42
    UnitQuaternion::from_quaternion(orientation + angular_rate_derivative * cycle_time)
}

fn correct(
    orientation: UnitQuaternion<f32>,
    measured_acceleration: Vector3<f32>,
    acceleration_weight: f32,
) -> Option<UnitQuaternion<f32>> {
    let measured_acceleration = measured_acceleration.normalize();
    // Equation 60
    let magnitude_error =
        (measured_acceleration.norm() - GRAVITATIONAL_CONSTANT).abs() / GRAVITATIONAL_CONSTANT;
    // Figure 5
    let interpolation_factor = if magnitude_error <= 0.1 {
        acceleration_weight
    } else if magnitude_error <= 0.2 {
        10.0 * acceleration_weight * (0.2 - magnitude_error)
    } else {
        return None;
    };
    // Equation 44
    let projected_gravity = orientation
        .inverse()
        .transform_vector(&measured_acceleration);
    // Equation 47
    let intermediate = ((projected_gravity.z + 1.0) * 0.5).sqrt();
    let acceleration_delta = UnitQuaternion::from_quaternion(Quaternion::new(
        intermediate,
        -projected_gravity.y / (2.0 * intermediate),
        projected_gravity.z / (2.0 * intermediate),
        0.0,
    ));
    // Equations 48, 49, 50, 51, 52
    const ANGLE_THRESHOLD: f32 = 0.9;
    let interpolated_acceleration_delta =
        if Quaternion::identity().dot(&acceleration_delta) > ANGLE_THRESHOLD {
            UnitQuaternion::from_quaternion(
                UnitQuaternion::identity().lerp(&acceleration_delta, interpolation_factor),
            )
        } else {
            UnitQuaternion::identity().slerp(&acceleration_delta, interpolation_factor)
        };

    // Equation 53
    Some(interpolated_acceleration_delta)
}

fn is_in_steady_state(
    previous_angular_velocity: Vector3<f32>,
    measured_acceleration: Vector3<f32>,
    measured_angular_velocity: Vector3<f32>,
    left_force_sensitive_resistor: f32,
    right_force_sensitive_resistor: f32,
    parameters: &Parameters,
) -> bool {
    if (measured_acceleration.norm() - GRAVITATIONAL_CONSTANT).abs()
        > parameters.acceleration_threshold
    {
        return false;
    }

    let angular_velocity_delta = (measured_angular_velocity - previous_angular_velocity).abs();
    if angular_velocity_delta.x > parameters.delta_angular_velocity_threshold
        || angular_velocity_delta.y > parameters.delta_angular_velocity_threshold
        || angular_velocity_delta.z > parameters.delta_angular_velocity_threshold
    {
        return false;
    }

    if left_force_sensitive_resistor < parameters.force_sensitive_resistor_threshold
        || right_force_sensitive_resistor < parameters.force_sensitive_resistor_threshold
    {
        return false;
    }
    true
}

fn calculate_initial_orientation(linear_acceleration: Vector3<f32>) -> UnitQuaternion<f32> {
    let normalized_acceleration = linear_acceleration.normalize();
    // Equation 25
    if normalized_acceleration.z >= 0.0 {
        let intermediate = ((normalized_acceleration.z + 1.0) * 0.5).sqrt();
        UnitQuaternion::from_quaternion(Quaternion::new(
            intermediate,
            -normalized_acceleration.y / (2.0 * intermediate),
            normalized_acceleration.x / (2.0 * intermediate),
            0.0,
        ))
    } else {
        let intermediate = ((-normalized_acceleration.z + 1.0) * 0.5).sqrt();
        UnitQuaternion::from_quaternion(Quaternion::new(
            -normalized_acceleration.y / (2.0 * intermediate),
            intermediate,
            0.0,
            normalized_acceleration.x / (2.0 * intermediate),
        ))
    }
}

#[cfg(test)]
mod test {
    use std::f32::consts::PI;

    use approx::assert_relative_eq;
    use nalgebra::vector;
    use rand::{distributions::Distribution, prelude::StdRng, SeedableRng};
    use rand_distr::Normal;

    use super::*;

    const ACCELERATION_THRESHOLD: f32 = 0.2;
    const DELTA_ANGULAR_VELOCITY_THRESHOLD: f32 = 0.1;
    const ANGULAR_VELOCITY_BIAS_WEIGHT: f32 = 0.01;
    const ACCELERATION_WEIGHT: f32 = 0.01;
    const FALLING_THRESHOLD: f32 = 1.0;
    const FORCE_SENSITIVE_RESISTOR_THRESHOLD: f32 = 4.0;

    const NUMBER_OF_MEASUREMENTS: usize = 100;
    const CYCLE_TIME: f32 = 1.0 / (NUMBER_OF_MEASUREMENTS as f32);
    const SEED: u64 = 3;

    const PARAMETERS: Parameters = Parameters {
        acceleration_threshold: ACCELERATION_THRESHOLD,
        delta_angular_velocity_threshold: DELTA_ANGULAR_VELOCITY_THRESHOLD,
        angular_velocity_bias_weight: ANGULAR_VELOCITY_BIAS_WEIGHT,
        acceleration_weight: ACCELERATION_WEIGHT,
        falling_threshold: FALLING_THRESHOLD,
        force_sensitive_resistor_threshold: FORCE_SENSITIVE_RESISTOR_THRESHOLD,
    };

    fn get_noise(random_number_generator: &mut StdRng, standard_deviation: f32) -> Vector3<f32> {
        let normal = Normal::new(0.0, standard_deviation).unwrap();
        vector![
            normal.sample(random_number_generator),
            normal.sample(random_number_generator),
            normal.sample(random_number_generator)
        ]
    }

    #[test]
    fn half_rotation() {
        let mut measured_acceleration = Vec::new();
        let mut measured_angular_velocity = Vec::new();
        let frequency = PI;
        for _ in 0..=NUMBER_OF_MEASUREMENTS {
            measured_acceleration.push(vector![0.0, 0.0, GRAVITATIONAL_CONSTANT]);
            measured_angular_velocity.push(frequency * Vector3::z());
        }
        let mut state = State::default();
        for i in 0..=NUMBER_OF_MEASUREMENTS {
            state.update(
                measured_acceleration[i],
                measured_angular_velocity[i],
                0.0,
                0.0,
                CYCLE_TIME,
                &PARAMETERS,
            );
        }
        assert_relative_eq!(state.yaw().angle(), PI, epsilon = 1e-3);
    }

    #[test]
    fn half_rotation_with_noise() {
        const NOISE_STANDARD_DEVIATION: f32 = 1.0;

        let mut random_number_generator = StdRng::seed_from_u64(SEED);

        let mut measured_acceleration = Vec::new();
        let mut measured_angular_velocity = Vec::new();
        let frequency = PI;
        for _ in 0..=NUMBER_OF_MEASUREMENTS {
            measured_acceleration.push(
                vector![0.0, 0.0, GRAVITATIONAL_CONSTANT]
                    + get_noise(&mut random_number_generator, NOISE_STANDARD_DEVIATION),
            );
            measured_angular_velocity.push(
                frequency * Vector3::z()
                    + get_noise(&mut random_number_generator, NOISE_STANDARD_DEVIATION),
            );
        }

        let mut state = State::default();
        for i in 0..=NUMBER_OF_MEASUREMENTS {
            state.update(
                measured_acceleration[i],
                measured_angular_velocity[i],
                0.0,
                0.0,
                CYCLE_TIME,
                &PARAMETERS,
            );
        }
        assert_relative_eq!(state.yaw().angle(), -PI, epsilon = 1e-2);
    }

    #[test]
    fn full_rotation() {
        let mut measured_acceleration = Vec::new();
        let mut measured_angular_velocity = Vec::new();
        let frequency = TAU;
        for _ in 0..=NUMBER_OF_MEASUREMENTS {
            measured_acceleration.push(vector![0.0, 0.0, GRAVITATIONAL_CONSTANT]);
            measured_angular_velocity.push(frequency * Vector3::z());
        }
        let mut state = State::default();
        for i in 0..=NUMBER_OF_MEASUREMENTS {
            state.update(
                measured_acceleration[i],
                measured_angular_velocity[i],
                0.0,
                0.0,
                CYCLE_TIME,
                &PARAMETERS,
            );
        }
        assert_relative_eq!(state.yaw().angle(), 0.0, epsilon = 1e-2);
    }

    #[test]
    fn full_rotation_with_noise() {
        const NOISE_STANDARD_DEVIATION: f32 = 1.0;

        let mut random_number_generator = StdRng::seed_from_u64(SEED);

        let mut measured_acceleration = Vec::new();
        let mut measured_angular_velocity = Vec::new();
        let frequency = TAU;
        for _ in 0..NUMBER_OF_MEASUREMENTS {
            measured_acceleration.push(
                vector![0.0, 0.0, GRAVITATIONAL_CONSTANT]
                    + get_noise(&mut random_number_generator, NOISE_STANDARD_DEVIATION),
            );
            measured_angular_velocity.push(
                frequency * Vector3::z()
                    + get_noise(&mut random_number_generator, NOISE_STANDARD_DEVIATION),
            );
        }
        let mut state = State::default();
        for i in 0..NUMBER_OF_MEASUREMENTS {
            state.update(
                measured_acceleration[i],
                measured_angular_velocity[i],
                0.0,
                0.0,
                CYCLE_TIME,
                &PARAMETERS,
            );
        }
        assert_relative_eq!(state.yaw().angle(), 0.0, epsilon = 1e-1);
    }

    #[test]
    fn full_rotation_with_noise_and_bias() {
        const BIAS: f32 = 1.0;

        const NOISE_STANDARD_DEVIATION: f32 = 0.0005;

        let mut random_number_generator = StdRng::seed_from_u64(SEED);

        let mut measured_acceleration = Vec::new();
        let mut measured_angular_velocity = Vec::new();
        let mut left_force_sensitive_resistor = Vec::new();
        let mut right_force_sensitive_resistor = Vec::new();
        let frequency = TAU;

        for _ in 0..NUMBER_OF_MEASUREMENTS {
            measured_acceleration.push(
                vector![0.0, 0.0, GRAVITATIONAL_CONSTANT]
                    + get_noise(&mut random_number_generator, NOISE_STANDARD_DEVIATION),
            );
            measured_angular_velocity.push(
                get_noise(&mut random_number_generator, NOISE_STANDARD_DEVIATION)
                    + vector![BIAS, -BIAS, BIAS],
            );
            left_force_sensitive_resistor.push(5.0);
            right_force_sensitive_resistor.push(5.0);
        }

        for _ in 0..NUMBER_OF_MEASUREMENTS {
            measured_acceleration.push(
                vector![0.0, 0.0, GRAVITATIONAL_CONSTANT]
                    + get_noise(&mut random_number_generator, NOISE_STANDARD_DEVIATION),
            );
            measured_angular_velocity.push(
                frequency * Vector3::z()
                    + get_noise(&mut random_number_generator, NOISE_STANDARD_DEVIATION)
                    + vector![BIAS, -BIAS, BIAS],
            );
            left_force_sensitive_resistor.push(0.0);
            right_force_sensitive_resistor.push(0.0);
        }
        let mut state = State::default();
        for i in 0..2 * NUMBER_OF_MEASUREMENTS {
            state.update(
                measured_acceleration[i],
                measured_angular_velocity[i],
                5.0,
                5.0,
                CYCLE_TIME,
                &PARAMETERS,
            );
        }
        assert_relative_eq!(state.yaw().angle(), 0.0, epsilon = 1e-1);
    }
}
