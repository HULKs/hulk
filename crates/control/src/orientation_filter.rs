use ahrs::{Ahrs, Madgwick};
use color_eyre::{eyre::eyre, Result};
use filtering::low_pass_filter::LowPassFilter;
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
    beta: Parameter<f32, "orientation_filter.beta">,
    calibration_steady_threshold: Parameter<f32, "orientation_filter.calibration_steady_threshold">,
    calibration_smoothing_factor: Parameter<f32, "orientation_filter.calibration_smoothing_factor">,
    num_calibration_cycles: Parameter<usize, "orientation_filter.num_calibration_cycles">,
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
                        remaining_cycles: *context.num_calibration_cycles,
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
                        self.state = State::Filtering {
                            state: nalgebra::UnitQuaternion::look_at_rh(
                                &-filtered_gravity.state().inner,
                                &nalgebra::Vector3::y(),
                            ),
                        };
                    }
                } else {
                    self.state = State::WaitingForSteady;
                }
            }
            State::Filtering { state } => {
                let mut filter = Madgwick::new_with_quat(0.012, *context.beta, *state);
                filter
                    .update_imu(
                        &measured_angular_velocity.inner,
                        &measured_acceleration.inner,
                    )
                    .map_err(|error| eyre!("failed to update orientation filter: {error}"))?;
                *state = filter.quat;
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
