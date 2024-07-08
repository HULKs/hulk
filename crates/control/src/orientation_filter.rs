use color_eyre::Result;
use filtering::{low_pass_filter::LowPassFilter, madgwick::Madgwick};
use nalgebra::UnitQuaternion;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Robot};
use framework::MainOutput;
use linear_algebra::{IntoFramed, Orientation3, Vector3};
use types::sensor_data::SensorData;

#[derive(Default, Serialize, Deserialize)]
enum State {
    #[default]
    WaitingForSteady,
    CalibratingGravity {
        filtered_gravity: LowPassFilter<Vector3<Robot>>,
        remaining_cycles: usize,
    },
    Filtering {
        state: UnitQuaternion<f32>,
    },
}

#[derive(Default, Serialize, Deserialize)]
pub struct OrientationFilter {
    state: State,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
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

        match &mut self.state {
            State::WaitingForSteady => {
                if measured_angular_velocity.abs().inner.sum()
                    < *context.calibration_steady_threshold
                {
                    self.state = State::CalibratingGravity {
                        filtered_gravity: LowPassFilter::with_smoothing_factor(
                            measured_acceleration,
                            *context.calibration_smoothing_factor,
                        ),
                        remaining_cycles: *context.number_of_calibration_cycles,
                    }
                }
            }
            State::CalibratingGravity {
                filtered_gravity,
                remaining_cycles,
            } => {
                if measured_angular_velocity.abs().inner.sum()
                    < *context.calibration_steady_threshold
                {
                    filtered_gravity.update(measured_acceleration);
                    *remaining_cycles -= 1;
                    if *remaining_cycles == 0 {
                        let orientation = nalgebra::UnitQuaternion::look_at_rh(
                            &-filtered_gravity.state().inner,
                            &nalgebra::Vector3::y(),
                        );
                        self.state = State::Filtering { state: orientation };
                    }
                } else {
                    self.state = State::WaitingForSteady;
                }
            }
            State::Filtering { state } => {
                if state
                    .update_with_imu(
                        &measured_angular_velocity.inner,
                        &measured_acceleration.inner,
                        *context.filter_gain,
                        0.012,
                    )
                    .is_err()
                {
                    state.update_with_gyroscope(
                        &measured_angular_velocity.inner,
                        *context.filter_gain,
                    );
                }
            }
        }

        let orientation = match &self.state {
            State::Filtering { state } => Some((*state).framed()),
            _ => None,
        };

        Ok(MainOutputs {
            robot_orientation: orientation.into(),
        })
    }
}
