use color_eyre::Result;
use filtering::{low_pass_filter::LowPassFilter, madgwick::Madgwick};
use nalgebra::UnitQuaternion;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Robot};
use framework::MainOutput;
use linear_algebra::{IntoFramed, Orientation3, Vector3};
use types::{cycle_time::CycleTime, sensor_data::SensorData};

#[derive(
    Clone, Default, Serialize, Deserialize, PathDeserialize, PathSerialize, PathIntrospect,
)]
enum State {
    #[default]
    WaitingForSteady,
    CalibratingGravity {
        filtered_gravity: LowPassFilter<Vector3<Robot>>,
        number_of_cycles: usize,
    },
}

#[derive(Default, Serialize, Deserialize)]
pub struct OrientationFilter {
    calibration_state: State,
    calibration: Option<UnitQuaternion<f32>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    cycle_time: Input<CycleTime, "cycle_time">,
    sensor_data: Input<SensorData, "sensor_data">,
    filter_gain: Parameter<f32, "orientation_filter.filter_gain">,
    calibration_steady_threshold: Parameter<f32, "orientation_filter.calibration_steady_threshold">,
    calibration_smoothing_factor: Parameter<f32, "orientation_filter.calibration_smoothing_factor">,
    number_of_calibration_cycles:
        Parameter<usize, "orientation_filter.number_of_calibration_cycles">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub recalibrated_this_cycle: MainOutput<bool>,
    pub robot_orientation: MainOutput<Option<Orientation3<Field>>>,
}

impl OrientationFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Default::default())
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let measured_angular_velocity = context
            .sensor_data
            .inertial_measurement_unit
            .angular_velocity;
        let measured_acceleration = context
            .sensor_data
            .inertial_measurement_unit
            .linear_acceleration;

        let mut recalibrated_this_cycle = false;
        match &mut self.calibration_state {
            State::WaitingForSteady => {
                if measured_angular_velocity.abs().inner.sum()
                    < *context.calibration_steady_threshold
                {
                    self.calibration_state = State::CalibratingGravity {
                        filtered_gravity: LowPassFilter::with_smoothing_factor(
                            measured_acceleration,
                            *context.calibration_smoothing_factor,
                        ),
                        number_of_cycles: 0,
                    }
                }
            }
            State::CalibratingGravity {
                filtered_gravity,
                number_of_cycles,
            } => 'update: {
                if measured_angular_velocity.abs().inner.sum()
                    >= *context.calibration_steady_threshold
                {
                    self.calibration_state = State::WaitingForSteady;
                    break 'update;
                }

                filtered_gravity.update(measured_acceleration);
                *number_of_cycles += 1;

                if *number_of_cycles >= *context.number_of_calibration_cycles {
                    let starting_yaw = match self.calibration {
                        Some(calibration) => calibration.euler_angles().2,
                        None => 0.0,
                    };
                    let gravity = -filtered_gravity.state().inner;
                    let up = nalgebra::Vector3::y();
                    let (roll, pitch, _) = UnitQuaternion::look_at_rh(&gravity, &up).euler_angles();
                    let orientation = UnitQuaternion::from_euler_angles(roll, pitch, starting_yaw);

                    recalibrated_this_cycle = true;
                    self.calibration = Some(orientation);
                    *number_of_cycles = 0;
                }
            }
        };

        let orientation = self.calibration.as_mut().map(|calibration| {
            if calibration
                .update_with_imu(
                    measured_angular_velocity.inner,
                    measured_acceleration.inner,
                    *context.filter_gain,
                    context.cycle_time.last_cycle_duration,
                )
                .is_err()
            {
                calibration.update_with_gyroscope(
                    measured_angular_velocity.inner,
                    context.cycle_time.last_cycle_duration,
                );
            }

            (*calibration).framed()
        });

        Ok(MainOutputs {
            robot_orientation: orientation.into(),
            recalibrated_this_cycle: recalibrated_this_cycle.into(),
        })
    }
}
